//! 下载与安装，跑在下载线程里：清单 → 逐块下载并核对 → 核对整个压缩包 → 解到临时目录 → 换进模型目录。

use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use super::manifest::Manifest;
use super::pack::Pack;
use super::{Shared, Stage};

/// 两边的附件一模一样：同时取清单，先回来的那边先用（大陆多半是 Gitee，海外多半是 GitHub），不通再换另一边。
const HOSTS: [&str; 2] = [
    "https://gitee.com/qingjian-team/qingjian-models/releases/download",
    "https://github.com/qingjian-team/qingjian-models/releases/download",
];

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

/// 两次数据之间最多等这么久；整个下载不设上限，Gitee 限速时知意要一分多钟。
const READ_TIMEOUT: Duration = Duration::from_secs(30);

/// 进度最多这么久报一次，免得每个数据块都重画窗口。
const REPORT_EVERY: Duration = Duration::from_millis(100);

const MAX_MANIFEST_BYTES: usize = 64 * 1024;

pub(super) fn run(pack: Pack, user_dir: &Path, shared: &Shared) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("起不了下载：{error}"))?;
    let models = user_dir.join("models");
    let work = models.join(format!(".download-{}", pack.tag));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).map_err(|error| format!("建不了下载目录：{error}"))?;
    let result = runtime.block_on(download(pack, &work, shared));
    let result = result.and_then(|archive| {
        shared.set(Stage::Installing);
        install(pack, &archive, &work, user_dir)
    });
    let _ = std::fs::remove_dir_all(&work);
    result
}

/// 下载全部分块拼成一个压缩包，返回它的路径。
async fn download(pack: Pack, work: &Path, shared: &Shared) -> Result<std::path::PathBuf, String> {
    let client = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
        .user_agent(format!("qingjian/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("起不了下载：{error}"))?;
    let (manifest, hosts) = manifest(&client, pack).await?;
    let archive_path = work.join(format!("{}.tar.gz", pack.tag));
    let mut archive =
        File::create(&archive_path).map_err(|error| format!("写不了下载文件：{error}"))?;
    let mut whole = Sha256::new();
    let total = manifest.parts.iter().map(|part| part.bytes).sum();
    let mut done = 0;
    shared.set(Stage::Downloading { done, total });
    for part in &manifest.parts {
        let mut last_error = String::new();
        let mut fetched = None;
        for host in hosts {
            let url = format!("{host}/{}/{}", pack.tag, part.name);
            match fetch_part(&client, &url, part.bytes, done, total, shared).await {
                Ok(bytes) if sha256(&bytes) == part.sha256 => {
                    fetched = Some(bytes);
                    break;
                }
                Ok(_) => last_error = format!("{} 校验不对", part.name),
                Err(error) => last_error = error,
            }
            if shared.cancelled() {
                return Err(CANCELLED.to_owned());
            }
            tracing::warn!(url, error = last_error, "这一块没下好，换下一个地址");
        }
        let bytes = fetched.ok_or(last_error)?;
        whole.update(&bytes);
        archive
            .write_all(&bytes)
            .map_err(|error| format!("写不了下载文件：{error}"))?;
        done += part.bytes;
        shared.set(Stage::Downloading { done, total });
    }
    archive
        .flush()
        .map_err(|error| format!("写不了下载文件：{error}"))?;
    if hex(&whole.finalize()) != manifest.archive.sha256 {
        return Err("拼起来的压缩包校验不对".to_owned());
    }
    Ok(archive_path)
}

pub(super) const CANCELLED: &str = "已取消";

/// 两边同时取清单，返回先取到的那份和接下来下分块的顺序（先回来的在前）；一边失败就等另一边。
async fn manifest(
    client: &reqwest::Client,
    pack: Pack,
) -> Result<(Manifest, [&'static str; 2]), String> {
    let [first, second] = HOSTS;
    let from_first = manifest_from(client, first, pack);
    let from_second = manifest_from(client, second, pack);
    tokio::pin!(from_first, from_second);
    tokio::select! {
        result = &mut from_first => match result {
            Ok(manifest) => Ok((manifest, [first, second])),
            Err(_) => from_second.await.map(|manifest| (manifest, [second, first])),
        },
        result = &mut from_second => match result {
            Ok(manifest) => Ok((manifest, [second, first])),
            Err(_) => from_first.await.map(|manifest| (manifest, [first, second])),
        },
    }
}

/// 一边的清单，标签与大小要对得上。
async fn manifest_from(
    client: &reqwest::Client,
    host: &str,
    pack: Pack,
) -> Result<Manifest, String> {
    let url = format!("{host}/{}/manifest.json", pack.tag);
    let result = match get_small(client, &url).await {
        Ok(bytes) => match serde_json::from_slice::<Manifest>(&bytes) {
            Ok(manifest) if manifest.tag != pack.tag || manifest.parts.is_empty() => {
                Err("下载清单不是这个版本的".to_owned())
            }
            Ok(manifest) => {
                let sum: u64 = manifest.parts.iter().map(|part| part.bytes).sum();
                if sum == manifest.archive.bytes {
                    Ok(manifest)
                } else {
                    Err("下载清单的大小对不上".to_owned())
                }
            }
            Err(error) => Err(format!("下载清单读不懂：{error}")),
        },
        Err(error) => Err(error),
    };
    if let Err(error) = &result {
        tracing::warn!(url, error, "下载清单没取到");
    }
    result
}

async fn get_small(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let response = client
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(describe)?;
    let bytes = response.bytes().await.map_err(describe)?;
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err("下载清单太大".to_owned());
    }
    Ok(bytes.to_vec())
}

/// 下一块，边下边报进度（`before` 是这块之前已下好的字节数）；超过清单写的大小就停。
async fn fetch_part(
    client: &reqwest::Client,
    url: &str,
    expected: u64,
    before: u64,
    total: u64,
    shared: &Shared,
) -> Result<Vec<u8>, String> {
    let mut response = client
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(describe)?;
    let mut bytes = Vec::with_capacity(expected as usize);
    let mut reported = Instant::now();
    while let Some(chunk) = response.chunk().await.map_err(describe)? {
        if shared.cancelled() {
            return Err(CANCELLED.to_owned());
        }
        bytes.extend_from_slice(&chunk);
        if bytes.len() as u64 > expected {
            return Err("下到的比清单写的大".to_owned());
        }
        if reported.elapsed() >= REPORT_EVERY {
            reported = Instant::now();
            shared.set(Stage::Downloading {
                done: before + bytes.len() as u64,
                total,
            });
        }
    }
    Ok(bytes)
}

/// 解到临时目录，再把旧的模型目录换掉；中途失败不动原来装着的。
fn install(pack: Pack, archive: &Path, work: &Path, user_dir: &Path) -> Result<(), String> {
    let extracted = work.join("extracted");
    let file = File::open(archive).map_err(|error| format!("打不开下载文件：{error}"))?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(BufReader::new(file)));
    // unpack 不放行绝对路径与 `..`，解不出包外
    tar.unpack(&extracted)
        .map_err(|error| format!("解压失败：{error}"))?;
    let source = pack.archive_path(&extracted);
    if !source.is_dir() {
        return Err("压缩包里没有模型目录".to_owned());
    }
    let target = pack.install_path(user_dir);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|error| format!("建不了模型目录：{error}"))?;
    }
    let old = work.join("old");
    if target.exists() {
        std::fs::rename(&target, &old).map_err(|error| format!("挪不开旧模型：{error}"))?;
    }
    if let Err(error) = std::fs::rename(&source, &target) {
        let _ = std::fs::rename(&old, &target);
        return Err(format!("放不进模型目录：{error}"));
    }
    Ok(())
}

fn describe(error: reqwest::Error) -> String {
    if error.is_timeout() {
        "连接超时".to_owned()
    } else if let Some(status) = error.status() {
        format!("服务器返回 {}", status.as_u16())
    } else if error.is_connect() {
        "连不上服务器".to_owned()
    } else {
        format!("网络出错：{error}")
    }
}

fn sha256(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::sync::atomic::AtomicBool;

    use qingjian_platform::LocalModel;

    use super::*;

    #[test]
    #[ignore = "联网从模型仓库下 52 MB"]
    fn downloads_and_installs_zhiwei() {
        let dir = std::env::temp_dir().join("qingjian-model-download-test");
        let _ = std::fs::remove_dir_all(&dir);
        let shared = Shared {
            stage: Mutex::new(Stage::Installing),
            cancel: AtomicBool::new(false),
            waker: None,
        };
        let pack = Pack::of(LocalModel::Zhiwei).unwrap();
        run(pack, &dir, &shared).unwrap();
        let installed = pack.install_path(&dir);
        assert!(installed.join("hanzhang-zhiwei-small.qjm").is_file());
        assert!(installed.join("LICENSE").is_file());
        assert!(!dir.join("models/.download-hanzhang-zhiwei-v1.0.0").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn hashes_as_lowercase_hex() {
        assert_eq!(
            sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn reads_the_manifest_pack_sh_writes() {
        let text = r#"{
  "model": "zhiyi",
  "version": "1.0.0",
  "tag": "zhiyi-v1.0.0",
  "archive": {"bytes": 3, "sha256": "aa"},
  "parts": [
    {"name": "zhiyi-v1.0.0.tar.gz.part1", "bytes": 2, "sha256": "bb"},
    {"name": "zhiyi-v1.0.0.tar.gz.part2", "bytes": 1, "sha256": "cc"}
  ]
}"#;
        let manifest: Manifest = serde_json::from_str(text).unwrap();
        assert_eq!(manifest.tag, "zhiyi-v1.0.0");
        assert_eq!(manifest.archive.bytes, 3);
        assert_eq!(manifest.parts[1].name, "zhiyi-v1.0.0.tar.gz.part2");
    }
}
