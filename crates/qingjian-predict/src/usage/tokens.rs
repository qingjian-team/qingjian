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
