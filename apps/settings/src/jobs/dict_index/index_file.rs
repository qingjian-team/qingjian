//! `index.json` 本身：清单格式版本与各本词库。

use serde::Deserialize;

use super::DictEntry;

use crate::i18n::{tr, tr_args};

/// 这一版输入法读得懂的清单格式；词库仓库改了清单的写法就加一，旧输入法提示更新。
const INDEX_FORMAT: u32 = 1;

#[derive(Debug, Deserialize)]
pub(super) struct IndexFile {
    format: u32,

    dicts: Vec<DictEntry>,
}

impl IndexFile {
    pub(super) fn parse(bytes: &[u8]) -> Result<Vec<DictEntry>, String> {
        let file: Self = serde_json::from_slice(bytes)
            .map_err(|error| tr_args("download.bad_index", &[("error", &error)]))?;
        if file.format != INDEX_FORMAT {
            return Err(tr("download.index_too_new").to_owned());
        }
        Ok(file
            .dicts
            .into_iter()
            .filter(|entry| {
                DictEntry::listed(entry) && safe_component(&entry.id) && safe_component(&entry.tag)
            })
            .collect())
    }
}

/// `id` 会拼成 `downloads/dicts/<id>.qj`，`tag` 会拼成临时文件名与下载 URL。
/// 只接受字母数字与 `-` `_` `.`：空、`.`、`..` 以及带路径分隔符的一律丢掉，
/// 免得远端一份清单把解包写到下载目录外面。
fn safe_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value != "."
        && value != ".."
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

#[cfg(test)]
mod tests {
    use qingjian_format::Kind;

    use super::*;
    use crate::jobs::download::Install;

    #[test]
    fn keeps_extra_dictionaries_and_known_glossaries_only() {
        let text = r#"{
  "format": 1,
  "dicts": [
    {"id": "domain-medicine", "name": "青简领域词库·医学", "kind": "extra", "version": "1.0.0",
     "tag": "domain-medicine-v1.0.0", "bytes": 327592, "qj_format": 1, "license": "MIT AND CC-BY-SA-4.0",
     "description": "医学", "official": true, "author": {"name": "青简", "url": "https://qingjian.app"}},
    {"id": "glossary-ja", "name": "日语", "kind": "glossary", "language": "ja", "version": "1.0.0",
     "tag": "glossary-ja-v1.0.0", "bytes": 1, "qj_format": 1, "license": "CC-BY-SA-4.0"},
    {"id": "glossary-fr", "name": "法语", "kind": "glossary", "language": "fr", "version": "1.0.0",
     "tag": "glossary-fr-v1.0.0", "bytes": 1, "qj_format": 1, "license": "CC-BY-SA-4.0"},
    {"id": "later", "name": "以后的种类", "kind": "theme", "version": "1.0.0",
     "tag": "later-v1.0.0", "bytes": 1, "qj_format": 1, "license": "CC0-1.0"},
    {"id": "base", "name": "基础", "kind": "base", "version": "1.0.0",
     "tag": "base-v1.0.0", "bytes": 1, "qj_format": 1, "license": "CC-BY-SA-4.0"},
    {"id": "future", "name": "未来", "kind": "extra", "version": "1.0.0",
     "tag": "future-v1.0.0", "bytes": 1, "qj_format": 99, "license": "CC0-1.0", "unknown_field": 1}
  ]
}"#;
        let dicts = IndexFile::parse(text.as_bytes()).unwrap();
        let ids: Vec<&str> = dicts.iter().map(|entry| entry.id.as_str()).collect();
        assert_eq!(ids, ["domain-medicine", "glossary-ja", "future"]);
        assert!(dicts[0].official && !dicts[0].needs_newer_app());
        assert_eq!(dicts[0].author.link(), Some("https://qingjian.app"));
        assert_eq!(dicts[1].author.name, "");
        assert!(dicts[2].needs_newer_app());
        assert_eq!(dicts[0].target().tag, "domain-medicine-v1.0.0");
        let Install::Qjz { dest, kind } = dicts[0].target().install else {
            panic!("dictionary installs from a .qjz");
        };
        assert_eq!(
            (dest.as_str(), kind),
            ("dicts/domain-medicine.qj", Kind::Dictionary)
        );
        let Install::Qjz { dest, kind } = dicts[1].target().install else {
            panic!("glossary installs from a .qjz");
        };
        assert_eq!(
            (dest.as_str(), kind),
            ("glossaries/glossary-ja.qj", Kind::Glossary)
        );
    }

    #[test]
    fn rejects_unknown_index_format() {
        assert!(IndexFile::parse(br#"{"format": 2, "dicts": []}"#).is_err());
        assert!(IndexFile::parse(b"not json").is_err());
    }

    /// 清单来自网上，`id` / `tag` 会进本地路径；带 `..` 或分隔符的条目必须丢掉。
    #[test]
    fn drops_entries_with_unsafe_path_components() {
        let text = r#"{
  "format": 1,
  "dicts": [
    {"id": "../../evil", "name": "x", "kind": "extra", "version": "1.0.0",
     "tag": "evil-v1.0.0", "bytes": 1, "qj_format": 1, "license": "MIT"},
    {"id": "escape", "name": "x", "kind": "extra", "version": "1.0.0",
     "tag": "../escape", "bytes": 1, "qj_format": 1, "license": "MIT"},
    {"id": "slash", "name": "x", "kind": "extra", "version": "1.0.0",
     "tag": "a/b", "bytes": 1, "qj_format": 1, "license": "MIT"},
    {"id": "ok", "name": "x", "kind": "extra", "version": "1.0.0",
     "tag": "ok-v1.0.0", "bytes": 1, "qj_format": 1, "license": "MIT"}
  ]
}"#;
        let dicts = IndexFile::parse(text.as_bytes()).unwrap();
        assert_eq!(dicts.len(), 1);
        assert_eq!(dicts[0].id, "ok");
    }
}
