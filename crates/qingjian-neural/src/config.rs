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
