//! `response_format` 的几级写法与降级顺序。

use serde_json::{Value, json};

use super::ReplySchema;

/// `response_format` 发哪种，按降级顺序排。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplyFormat {
    JsonObject,
    JsonSchema,
    /// 不发 `response_format`，全凭提示词让模型回 JSON。
    Unconstrained,
}

impl ReplyFormat {
    pub fn from_index(index: u8) -> Self {
        match index {
            0 => Self::JsonObject,
            1 => Self::JsonSchema,
            _ => Self::Unconstrained,
        }
    }

    pub fn index(self) -> u8 {
        self as u8
    }

    /// 被拒之后降到哪一级；已经最低就没有。
    pub fn fallback(self) -> Option<Self> {
        match self {
            Self::JsonObject => Some(Self::JsonSchema),
            Self::JsonSchema => Some(Self::Unconstrained),
            Self::Unconstrained => None,
        }
    }

    /// 把这一级的 `response_format` 写进请求体。
    pub fn apply(self, body: &mut Value, schema: &ReplySchema) {
        let Some(fields) = body.as_object_mut() else {
            return;
        };
        match self {
            Self::JsonObject => {
                fields.insert(
                    "response_format".to_owned(),
                    json!({ "type": "json_object" }),
                );
            }
            Self::JsonSchema => {
                fields.insert(
                    "response_format".to_owned(),
                    json!({
                        "type": "json_schema",
                        "json_schema": { "name": schema.name, "schema": schema.schema },
                    }),
                );
            }
            Self::Unconstrained => {
                fields.remove("response_format");
            }
        }
    }
}
