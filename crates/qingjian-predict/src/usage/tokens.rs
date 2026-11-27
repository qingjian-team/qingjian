//! 从 OpenAI 兼容响应提取实际用量；缓存输入属于输入总量，不重复累加。

use serde_json::Value;

pub(super) struct UsageTokens {
    pub input: u64,

    pub output: u64,

    pub total: u64,

    pub cached_input: u64,
}

impl UsageTokens {
    pub fn from_response(response: &Value) -> Option<Self> {
        let usage = response.get("usage")?;
        let input = usage.get("prompt_tokens")?.as_u64()?;
        let output = usage.get("completion_tokens")?.as_u64()?;
        let total = usage
            .get("total_tokens")
            .and_then(Value::as_u64)
            .or_else(|| input.checked_add(output))?;
        let cached_input = usage
            .get("prompt_cache_hit_tokens")
            .and_then(Value::as_u64)
            .or_else(|| {
                usage
                    .pointer("/prompt_tokens_details/cached_tokens")
                    .and_then(Value::as_u64)
            })
            .unwrap_or(0);
        Some(Self {
            input,
            output,
            total,
            cached_input,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::UsageTokens;
    use serde_json::json;

    #[test]
    fn reads_the_standard_usage() {
        let usage = UsageTokens::from_response(&json!({
            "usage": {"prompt_tokens": 10, "completion_tokens": 4, "total_tokens": 14}
        }))
        .unwrap();
        assert_eq!(usage.input, 10);
        assert_eq!(usage.output, 4);
        assert_eq!(usage.total, 14);
        assert_eq!(usage.cached_input, 0);
    }

    #[test]
    fn total_falls_back_to_input_plus_output() {
        let usage = UsageTokens::from_response(&json!({
            "usage": {"prompt_tokens": 10, "completion_tokens": 4}
        }))
        .unwrap();
        assert_eq!(usage.total, 14);
    }

    #[test]
    fn cached_input_reads_either_field() {
        let deepseek = UsageTokens::from_response(&json!({
            "usage": {"prompt_tokens": 10, "completion_tokens": 1, "prompt_cache_hit_tokens": 8}
        }))
        .unwrap();
        assert_eq!(deepseek.cached_input, 8);
        let openai = UsageTokens::from_response(&json!({
            "usage": {"prompt_tokens": 10, "completion_tokens": 1,
                      "prompt_tokens_details": {"cached_tokens": 6}}
        }))
        .unwrap();
        assert_eq!(openai.cached_input, 6);
    }

    #[test]
    fn a_response_without_usage_has_none() {
        assert!(UsageTokens::from_response(&json!({})).is_none());
        assert!(UsageTokens::from_response(&json!({"usage": null})).is_none());
    }

    #[test]
    fn missing_or_non_numeric_counts_are_rejected() {
        assert!(UsageTokens::from_response(&json!({"usage": {"prompt_tokens": 10}})).is_none());
        assert!(
            UsageTokens::from_response(
                &json!({"usage": {"prompt_tokens": "10", "completion_tokens": 1}})
            )
            .is_none()
        );
    }
}
