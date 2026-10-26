//! 服务端回包的 JSON（只取用得上的字段）。

use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct Reply {
    #[serde(default)]
    pub result: Option<ReplyResult>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ReplyResult {
    /// 到目前为止整段的识别文字（`result_type` 为 `full`）。
    #[serde(default)]
    pub text: String,
}

impl Reply {
    pub fn text(&self) -> &str {
        self.result
            .as_ref()
            .map_or("", |result| result.text.as_str())
    }
}
