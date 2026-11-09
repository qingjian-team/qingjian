//! 回复格式：要模型回 JSON 时怎么跟接口说。
//!
//! 各家 OpenAI 兼容接口认的 `response_format` 不一样：多数认 `json_object`（DeepSeek 只认它），
//! LM Studio 只认 `json_schema`，还有的本地服务两样都不认。先发 `json_object`，被拒就降一级重发，
//! 客户端记住降到哪一级，之后的请求直接用。

mod format;
mod schema;

pub use format::ReplyFormat;
pub use schema::ReplySchema;

use serde_json::{Value, json};

/// 报错是不是在说回复格式不支持。LM Studio 的报错体是 `{"error": "..."}` 字符串，不合 OpenAI 的错误结构，
/// 会落到反序列化失败那一支，原文只在错误信息里，所以只看文字。
pub fn rejects_reply_format(error: &async_openai::error::OpenAIError) -> bool {
    let message = error.to_string();
    ["response_format", "json_object", "json_schema"]
        .iter()
        .any(|field| message.contains(field))
}

/// 不约束格式时，不少模型会把 JSON 包在 Markdown 代码块里，剥掉外层围栏。
pub fn unfenced(content: &str) -> &str {
    let trimmed = content.trim();
    let Some(inner) = trimmed
        .strip_prefix("```")
        .and_then(|rest| rest.strip_suffix("```"))
    else {
        return trimmed;
    };
    // 开头围栏后面可能跟语言名（```json），到第一个换行为止都不是正文
    match inner.split_once('\n') {
        Some((_, body)) => body.trim(),
        None => inner.trim(),
    }
}

/// 只含给定字段、字段全部必填的对象。
pub fn object(properties: Value, required: &[&str]) -> Value {
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> ReplySchema {
        ReplySchema::new(
            "test",
            object(json!({ "ok": { "type": "boolean" } }), &["ok"]),
        )
    }

    #[test]
    fn formats_fall_back_in_order_and_stop_at_unconstrained() {
        let mut format = ReplyFormat::JsonObject;
        let mut seen = vec![format];
        while let Some(next) = format.fallback() {
            format = next;
            seen.push(format);
        }
        assert_eq!(
            seen,
            [
                ReplyFormat::JsonObject,
                ReplyFormat::JsonSchema,
                ReplyFormat::Unconstrained
            ]
        );
        for format in seen {
            assert_eq!(ReplyFormat::from_index(format.index()), format);
        }
    }

    #[test]
    fn each_format_writes_its_own_response_format() {
        let mut body = json!({ "model": "m" });
        ReplyFormat::JsonObject.apply(&mut body, &schema());
        assert_eq!(body["response_format"], json!({ "type": "json_object" }));

        ReplyFormat::JsonSchema.apply(&mut body, &schema());
        assert_eq!(body["response_format"]["type"], "json_schema");
        assert_eq!(body["response_format"]["json_schema"]["name"], "test");
        assert_eq!(
            body["response_format"]["json_schema"]["schema"]["required"],
            json!(["ok"])
        );

        ReplyFormat::Unconstrained.apply(&mut body, &schema());
        assert!(body.get("response_format").is_none());
    }

    /// 回归 #515：LM Studio 的字符串报错体能认出来，思考类报错不误判。
    #[test]
    fn format_rejection_is_detected_from_the_message() {
        let lm_studio = async_openai::error::OpenAIError::JSONDeserialize(
            serde_json::from_str::<async_openai::error::ApiError>("\"x\"").unwrap_err(),
            r#"{"error":"'response_format.type' must be 'json_schema' or 'text'"}"#.to_owned(),
        );
        assert!(rejects_reply_format(&lm_studio));
        let thinking = async_openai::error::OpenAIError::InvalidArgument(
            "该模型始终思考，不支持关闭思考".to_owned(),
        );
        assert!(!rejects_reply_format(&thinking));
    }

    #[test]
    fn fenced_json_is_unwrapped() {
        assert_eq!(
            unfenced("```json\n{\"words\": []}\n```"),
            r#"{"words": []}"#
        );
        assert_eq!(unfenced("```\n{}\n```"), "{}");
        assert_eq!(unfenced("  {\"a\": 1}  "), r#"{"a": 1}"#);
    }
}
