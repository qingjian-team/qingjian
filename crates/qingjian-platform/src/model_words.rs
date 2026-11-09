//! 从知意三元模型词表补出的词（[`qingjian_dictionary::supplement`]）：第一次接上模型时在本机生成、缓存到模型旁边，
//! 之后按模型与主词库的大小、修改时间核对，对得上就直接映射缓存。派生数据只在用户机器上，不随包分发。

use std::path::{Path, PathBuf};
use std::time::{Instant, UNIX_EPOCH};

use qingjian_dictionary::Dictionary;
use qingjian_format::Metadata;

/// 生成规则改了就加一：旧缓存的戳对不上，下次启动重建。
const RULES_VERSION: u32 = 1;

/// 缓存放在模型旁边：`zh.ngram` → `zh.words.qj`。
pub fn cache_path(model: &Path) -> PathBuf {
    model.with_extension("words.qj")
}

/// 这几个文件的大小与修改时间连同规则版本拼成的戳，任一变了缓存就作废。
pub fn stamp(sources: &[&Path]) -> String {
    let mut parts = vec![format!("v{RULES_VERSION}")];
    for path in sources {
        let metadata = std::fs::metadata(path).ok();
        let modified = metadata
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs());
        parts.push(format!("{}-{modified}", metadata.map_or(0, |m| m.len())));
    }
    parts.join("/")
}

/// 缓存的戳对得上就用缓存，否则 `build` 现建、写回缓存（写不了只记日志，下次再建）。结果标成补充词库。
pub fn load(
    cache: &Path,
    stamp: &str,
    build: impl FnOnce() -> Option<Dictionary>,
) -> Option<Dictionary> {
    let cached = cache
        .is_file()
        .then(|| Dictionary::from_path(cache).ok())
        .flatten()
        .filter(|d| d.metadata().is_some_and(|m| m.version == stamp));
    let mut dictionary = match cached {
        Some(dictionary) => dictionary,
        None => {
            let started = Instant::now();
            let dictionary = build()?;
            tracing::info!(
                entries = dictionary.len(),
                build_ms = started.elapsed().as_millis(),
                "三元模型词表补词已生成"
            );
            let metadata = Metadata {
                name: "知意三元模型词表补词".to_owned(),
                license: "LGPL-2.1-or-later".to_owned(),
                version: stamp.to_owned(),
                ..Metadata::default()
            };
            // 先写临时文件再改名：装机时可能两个 Server 同时起、同时建，各写各的临时文件，改名是整个换，读的一方不会读到写了一半的
            let partial = cache.with_extension(format!("qj.{}.tmp", std::process::id()));
            let written = dictionary
                .write_qj(&partial, &metadata)
                .map_err(|e| e.to_string())
                .and_then(|()| std::fs::rename(&partial, cache).map_err(|e| e.to_string()));
            if let Err(error) = written {
                let _ = std::fs::remove_file(&partial);
                tracing::warn!(path = %cache.display(), %error, "补词缓存写不了，下次启动重建");
            }
            dictionary
        }
    };
    dictionary.set_secondary(true);
    Some(dictionary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "qingjian-model-words-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_matching_stamp_reuses_the_cache_and_a_new_one_rebuilds() {
        let dir = temp("stamp");
        let cache = dir.join("zh.words.qj");
        let build = |text: &'static str| move || Dictionary::parse(text).ok();
        let first = load(&cache, "a", build("摸鱼\tmo yu\t10\n")).unwrap();
        assert!(first.is_secondary());
        // 戳一样：不调 build
        let again = load(&cache, "a", || panic!("不该重建")).unwrap();
        assert_eq!(again.lookup(&["mo", "yu"], false)[0].text, "摸鱼");
        // 戳变了：重建并覆盖缓存
        let rebuilt = load(&cache, "b", build("老梗\tlao geng\t10\n")).unwrap();
        assert_eq!(rebuilt.lookup(&["lao", "geng"], false)[0].text, "老梗");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_stamp_follows_the_source_files() {
        let dir = temp("sources");
        let file = dir.join("zh.ngram");
        std::fs::write(&file, b"abc").unwrap();
        let before = stamp(&[&file]);
        std::fs::write(&file, b"abcd").unwrap();
        assert_ne!(before, stamp(&[&file]));
        assert_eq!(cache_path(&file), dir.join("zh.words.qj"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
