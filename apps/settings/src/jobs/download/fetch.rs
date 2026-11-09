//! 下载，跑在下载线程里：清单 → 逐块下载并核对 → 核对拼起来的整个文件 → 交给 [`super::Install`] 装。
//! 取小文件的 [`fetch_small`] 也给 `dict_index` 取词库清单用。

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use super::Target;
use super::manifest::Manifest;
use super::shared::Shared;
use super::stage::Stage;

use crate::i18n::{tr, tr_args};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

/// 两次数据之间最多等这么久；整个下载不设上限，Gitee 限速时知意要一分多钟。
const READ_TIMEOUT: Duration = Duration::from_secs(30);

/// 进度最多这么久报一次，免得每个数据块都重画窗口。
const REPORT_EVERY: Duration = Duration::from_millis(100);

/// 清单与 `index.json` 最多这么大；词库清单一条几百字节，留得下几百本。
const MAX_SMALL_BYTES: usize = 512 * 1024;

pub(super) const CANCELLED: &str = "已取消";

pub(super) fn run(target: &Target, downloads: &Path, shared: &Shared) -> Result<(), String> {
    let runtime = runtime()?;
    let work = target.work_dir(downloads);
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work)
        .map_err(|error| tr_args("download.cant_create_dir", &[("error", &error)]))?;
    let result = runtime.block_on(download(target, &work, shared));
    let result = result.and_then(|archive| {
        shared.set(Stage::Installing);
        target.install.run(&archive, &work, downloads)
    });
    let _ = std::fs::remove_dir_all(&work);
    result
}

/// 同步版的 [`first_ok`]：几个地址同时取一个小文件，先取到且解析通过的用（给 `dict_index` 的线程用）。
pub fn fetch_small<T>(
    urls: Vec<String>,
    parse: impl Fn(&[u8]) -> Result<T, String>,
) -> Result<T, String> {
    let client = client()?;
    runtime()?
        .block_on(first_ok(&client, urls, parse))
        .map(|(value, _)| value)
}

fn runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| tr_args("download.cant_start", &[("error", &error)]))
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
        .user_agent(format!("qingjian/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| tr_args("download.cant_start", &[("error", &error)]))
}

/// 下载全部分块拼成一个文件，返回它的路径。
async fn download(target: &Target, work: &Path, shared: &Shared) -> Result<PathBuf, String> {
    let client = client()?;
    let hosts = target.repo.release_hosts();
    let urls = hosts
        .iter()
        .map(|host| format!("{host}/{}/manifest.json", target.tag))
        .collect();
    let (manifest, first) =
        first_ok(&client, urls, |bytes| check_manifest(bytes, &target.tag)).await?;
    // 先回来的那边在前，分块也先从它下
    let mut order = hosts.clone();
    order.rotate_left(first);
    let archive_path = work.join(format!("{}.download", target.tag));
    let mut archive = File::create(&archive_path)
        .map_err(|error| tr_args("download.cant_write", &[("error", &error)]))?;
    let mut whole = Sha256::new();
    let total = manifest.parts.iter().map(|part| part.bytes).sum();
    let mut done = 0;
    shared.set(Stage::Downloading { done, total });
    for part in &manifest.parts {
        let mut last_error = String::new();
        let mut fetched = None;
        for host in &order {
            let url = format!("{host}/{}/{}", target.tag, part.name);
            match fetch_part(&client, &url, part.bytes, done, total, shared).await {
                Ok(bytes) if sha256(&bytes) == part.sha256 => {
                    fetched = Some(bytes);
                    break;
                }
                Ok(_) => last_error = tr_args("download.bad_checksum", &[("name", &part.name)]),
                Err(error) => last_error = error,
            }
            if shared.cancelled() {
                return Err(CANCELLED.to_owned());
            }
            tracing::warn!(url, error = last_error, "part failed, trying the next host");
        }
        let bytes = fetched.ok_or(last_error)?;
        whole.update(&bytes);
        archive
            .write_all(&bytes)
            .map_err(|error| tr_args("download.cant_write", &[("error", &error)]))?;
        done += part.bytes;
        shared.set(Stage::Downloading { done, total });
    }
    archive
        .flush()
        .map_err(|error| tr_args("download.cant_write", &[("error", &error)]))?;
    if hex(&whole.finalize()) != manifest.archive.sha256 {
        return Err(tr("download.bad_archive").to_owned());
    }
    Ok(archive_path)
}

/// 清单的标签与大小要对得上。
fn check_manifest(bytes: &[u8], tag: &str) -> Result<Manifest, String> {
    match serde_json::from_slice::<Manifest>(bytes) {
        Ok(manifest) if manifest.tag != tag || manifest.parts.is_empty() => {
            Err(tr("download.wrong_manifest").to_owned())
        }
        Ok(manifest) => {
            let sum: u64 = manifest.parts.iter().map(|part| part.bytes).sum();
            if sum == manifest.archive.bytes {
                Ok(manifest)
            } else {
                Err(tr("download.manifest_size").to_owned())
            }
        }
        Err(error) => Err(tr_args("download.bad_manifest", &[("error", &error)])),
    }
}

/// 几个地址同时取一个小文件，返回第一个取到且 `parse` 通过的结果与它是第几个地址；全都失败时报最后一个错。
async fn first_ok<T>(
    client: &reqwest::Client,
    urls: Vec<String>,
    parse: impl Fn(&[u8]) -> Result<T, String>,
) -> Result<(T, usize), String> {
    let mut pending = tokio::task::JoinSet::new();
    for (index, url) in urls.into_iter().enumerate() {
        let client = client.clone();
        pending.spawn(async move { (index, get_small(&client, &url).await, url) });
    }
    let mut last_error = tr("download.unreachable").to_owned();
    while let Some(joined) = pending.join_next().await {
        let Ok((index, result, url)) = joined else {
            continue;
        };
        match result.and_then(|bytes| parse(&bytes)) {
            Ok(value) => return Ok((value, index)),
            Err(error) => {
                tracing::warn!(url, error, "small file not fetched");
                last_error = error;
            }
        }
    }
    Err(last_error)
}

async fn get_small(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let response = client
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(describe)?;
    let bytes = response.bytes().await.map_err(describe)?;
    if bytes.len() > MAX_SMALL_BYTES {
        return Err(tr("download.manifest_too_large").to_owned());
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
            return Err(tr("download.too_large").to_owned());
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

fn describe(error: reqwest::Error) -> String {
    if error.is_timeout() {
        tr("download.timeout").to_owned()
    } else if let Some(status) = error.status() {
        tr_args("download.status", &[("status", &status.as_u16())])
    } else if error.is_connect() {
        tr("download.unreachable").to_owned()
    } else {
        tr_args("download.network", &[("error", &error)])
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
    use qingjian_platform::LocalModel;

    use super::*;
    use crate::jobs::download::Pack;

    #[test]
    #[ignore = "联网从模型仓库下 43 MB"]
    fn downloads_and_installs_tongbian() {
        let dir = std::env::temp_dir().join("qingjian-model-download-test");
        let _ = std::fs::remove_dir_all(&dir);
        let shared = Shared::new(None);
        let pack = Pack::of(LocalModel::Tongbian).unwrap();
        run(&pack.target(), &dir, &shared).unwrap();
        let installed = pack.install_path(&dir);
        assert!(installed.join("hanzhang-tongbian-small.qjm").is_file());
        assert!(installed.join("LICENSE").is_file());
        assert!(
            !dir.join("models/.download-hanzhang-tongbian-v1.0.0")
                .exists()
        );
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
        let manifest = check_manifest(text.as_bytes(), "zhiyi-v1.0.0").unwrap();
        assert_eq!(manifest.archive.bytes, 3);
        assert_eq!(manifest.parts[1].name, "zhiyi-v1.0.0.tar.gz.part2");
        assert!(check_manifest(text.as_bytes(), "zhiyi-v2.0.0").is_err());
    }
}
