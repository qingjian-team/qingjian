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
        Ok(file.dicts.into_iter().filter(DictEntry::listed).collect())
    }
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
}
