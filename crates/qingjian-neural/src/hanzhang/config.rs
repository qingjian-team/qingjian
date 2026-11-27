use std::path::Path;

use serde::Deserialize;

use crate::NeuralError;

/// 模型结构，来自导出目录的 `config.json`（训练脚本 `common.py::ModelConfig` 原样写出）。
#[derive(Debug, Clone, Deserialize)]
pub struct ModelConfig {
    /// 字表大小。
    pub vocab_size: usize,

    /// 层数。
    pub n_layer: usize,

    /// 隐层宽度。
    pub n_embd: usize,

    /// 注意力头数。
    pub n_head: usize,

    /// 最长上下文（token 数），位置嵌入的行数。
    pub context: usize,
}

impl ModelConfig {
    /// 解析 `config.json` 的正文；`path` 只用来报错。
    pub(crate) fn from_json(text: &str, path: &Path) -> Result<Self, NeuralError> {
        serde_json::from_str(text).map_err(|source| NeuralError::Json {
            path: path.to_owned(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_every_field() {
        let config = ModelConfig::from_json(
            r#"{"vocab_size": 7000, "n_layer": 8, "n_embd": 512, "n_head": 8, "context": 128}"#,
            Path::new("config.json"),
        )
        .unwrap();
        assert_eq!(config.vocab_size, 7000);
        assert_eq!(config.n_layer, 8);
        assert_eq!(config.n_embd, 512);
        assert_eq!(config.n_head, 8);
        assert_eq!(config.context, 128);
    }

    #[test]
    fn missing_fields_are_json_errors() {
        let error =
            ModelConfig::from_json(r#"{"n_layer": 8}"#, Path::new("config.json")).unwrap_err();
        assert!(matches!(error, NeuralError::Json { .. }), "{error:?}");
    }
}
