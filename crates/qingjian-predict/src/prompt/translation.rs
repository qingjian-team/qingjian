//! 模型返回的单条译文形状；语言和原文在解析时核对。
use serde::Deserialize;

#[derive(Deserialize, Default)]
#[serde(default)]
pub(super) struct RawTranslation {
    pub source: String,

    pub language: String,

    pub text: String,
}
