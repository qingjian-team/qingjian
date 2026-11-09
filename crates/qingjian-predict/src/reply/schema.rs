//! 回复的 JSON 形状，降到 `json_schema` 时发给接口。

use serde_json::Value;

/// 一种回复的 JSON 形状，降到 `json_schema` 时发给接口；和提示词里写的输出格式一致。
pub struct ReplySchema {
    /// 接口要求的格式名，只能用字母、数字、`_`、`-`。
    pub name: &'static str,

    pub schema: Value,
}

impl ReplySchema {
    pub fn new(name: &'static str, schema: Value) -> Self {
        Self { name, schema }
    }
}
