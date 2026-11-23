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

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> ReplySchema {
        ReplySchema::new("candidates", json!({"type": "object"}))
    }

    #[test]
    fn index_round_trips() {
        for format in [
            ReplyFormat::JsonObject,
            ReplyFormat::JsonSchema,
            ReplyFormat::Unconstrained,
        ] {
            assert_eq!(ReplyFormat::from_index(format.index()), format);
        }
    }

    #[test]
    fn unknown_indexes_land_on_unconstrained() {
        assert_eq!(ReplyFormat::from_index(2), ReplyFormat::Unconstrained);
        assert_eq!(ReplyFormat::from_index(9), ReplyFormat::Unconstrained);
    }

    #[test]
    fn fallback_walks_down_and_stops() {
        assert_eq!(
            ReplyFormat::JsonObject.fallback(),
            Some(ReplyFormat::JsonSchema)
        );
        assert_eq!(
            ReplyFormat::JsonSchema.fallback(),
            Some(ReplyFormat::Unconstrained)
        );
        assert_eq!(ReplyFormat::Unconstrained.fallback(), None);
    }

    #[test]
    fn json_object_writes_the_short_form() {
        let mut body = json!({});
        ReplyFormat::JsonObject.apply(&mut body, &schema());
        assert_eq!(body["response_format"], json!({"type": "json_object"}));
    }

    #[test]
    fn json_schema_embeds_the_name_and_the_schema() {
        let mut body = json!({});
        ReplyFormat::JsonSchema.apply(&mut body, &schema());
        assert_eq!(body["response_format"]["type"], "json_schema");
        assert_eq!(body["response_format"]["json_schema"]["name"], "candidates");
        assert_eq!(
            body["response_format"]["json_schema"]["schema"],
            json!({"type": "object"})
        );
    }

    #[test]
    fn unconstrained_removes_the_field() {
        let mut body = json!({"response_format": {"type": "json_object"}});
        ReplyFormat::Unconstrained.apply(&mut body, &schema());
        assert!(body.get("response_format").is_none());
    }

    #[test]
    fn a_non_object_body_is_left_alone() {
        let mut body = json!("不是对象");
        ReplyFormat::JsonObject.apply(&mut body, &schema());
        assert_eq!(body, json!("不是对象"));
    }
}
