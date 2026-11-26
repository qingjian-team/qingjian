//! `index.json` 本身：清单格式版本与各个模型。

use serde::Deserialize;

use super::ModelEntry;

use crate::i18n::{tr, tr_args};

/// 这一版输入法读得懂的清单格式；模型仓库改了清单的写法就加一，旧输入法提示更新。
const INDEX_FORMAT: u32 = 1;

#[derive(Debug, Deserialize)]
pub(super) struct IndexFile {
    format: u32,

    models: Vec<ModelEntry>,
}

impl IndexFile {
    pub(super) fn parse(bytes: &[u8]) -> Result<Vec<ModelEntry>, String> {
        let file: Self = serde_json::from_slice(bytes)
            .map_err(|error| tr_args("download.bad_index", &[("error", &error)]))?;
        if file.format != INDEX_FORMAT {
            return Err(tr("download.index_too_new").to_owned());
        }
        Ok(file.models.into_iter().filter(ModelEntry::listed).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::download::Install;

    const TEXT: &str = r#"{
  "format": 1,
  "models": [
    {"id": "zhiyi", "name": "知意 1.4V2-Long-L12", "engine": "zhiyi", "engine_format": 1, "version": "1.0.0",
     "license": "Apache-2.0", "recommended": true,
     "variants": [{"runtime": "candle", "tag": "zhiyi-v1.0.0", "archive_dir": "zhiyi", "bytes": 10},
                  {"runtime": "onnx", "tag": "zhiyi-onnx-v1.0.0", "archive_dir": "zhiyi-onnx", "bytes": 5}]},
    {"id": "zhiyi-next", "name": "以后的结构", "engine": "zhiyi", "engine_format": 99, "version": "1.0.0",
     "license": "Apache-2.0", "deprecated": false, "unknown_field": 1,
     "variants": [{"runtime": "candle", "tag": "t", "archive_dir": "a", "bytes": 1},
                  {"runtime": "onnx", "tag": "t", "archive_dir": "a", "bytes": 1}]},
    {"id": "later", "name": "以后的引擎", "engine": "something", "engine_format": 1, "version": "1.0.0",
     "license": "MIT", "variants": [{"runtime": "candle", "tag": "t", "archive_dir": "a", "bytes": 1}]},
    {"id": "gpu-only", "name": "只有别的运行方式", "engine": "zhiyi", "engine_format": 1, "version": "1.0.0",
     "license": "MIT", "variants": [{"runtime": "tensorrt", "tag": "t", "archive_dir": "a", "bytes": 1}]},
    {"id": "../escape", "name": "坏 id", "engine": "zhiyi", "engine_format": 1, "version": "1.0.0",
     "license": "MIT", "variants": [{"runtime": "candle", "tag": "t", "archive_dir": "a", "bytes": 1},
                                    {"runtime": "onnx", "tag": "t", "archive_dir": "a", "bytes": 1}]}
  ]
}"#;

    #[test]
    fn keeps_models_this_app_can_list() {
        let models = IndexFile::parse(TEXT.as_bytes()).unwrap();
        let ids: Vec<&str> = models.iter().map(|entry| entry.id.as_str()).collect();
        assert_eq!(ids, ["zhiyi", "zhiyi-next"]);
        assert!(models[0].recommended && !models[0].needs_newer_app());
        assert!(models[1].needs_newer_app());
    }

    #[test]
    fn picks_the_package_for_this_platform_and_installs_under_the_id() {
        let models = IndexFile::parse(TEXT.as_bytes()).unwrap();
        let target = models[0].target().unwrap();
        let (tag, archive_dir, bytes) = if cfg!(windows) {
            ("zhiyi-onnx-v1.0.0", "zhiyi-onnx", 5)
        } else {
            ("zhiyi-v1.0.0", "zhiyi", 10)
        };
        assert_eq!(target.tag, tag);
        assert_eq!(models[0].bytes(), bytes);
        assert_eq!(
            target.install,
            Install::Archive {
                archive_dir: archive_dir.to_owned(),
                install_dir: "models/zhiyi".to_owned(),
            }
        );
    }

    #[test]
    fn refuses_a_newer_index_format() {
        assert!(IndexFile::parse(br#"{"format": 2, "models": []}"#).is_err());
    }
}
