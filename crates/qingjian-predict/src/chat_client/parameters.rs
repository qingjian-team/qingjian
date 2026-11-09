//! 记住接口明确拒绝的可选参数；每次降级都改变请求，后续请求直接使用兼容写法。

use std::sync::atomic::{AtomicU8, Ordering};

use async_openai::error::OpenAIError;
use reqwest::StatusCode;
use serde_json::Value;

const COMPLETION_TOKENS: u8 = 1;
const TEMPERATURE: u8 = 2;
const REASONING_EFFORT: u8 = 4;
const THINKING: u8 = 8;

/// 新 token 字段把推理也算在上限内；保留调用方额度，并预留有限的推理空间。
const REASONING_TOKENS: u64 = 4096;

const PARAMETERS: [(&str, u8); 4] = [
    ("max_tokens", COMPLETION_TOKENS),
    ("temperature", TEMPERATURE),
    ("reasoning_effort", REASONING_EFFORT),
    ("thinking", THINKING),
];

#[derive(Default)]
pub(super) struct RequestParameters {
    rejected: AtomicU8,
}

impl RequestParameters {
    pub fn apply(&self, body: &mut Value) {
        let Some(fields) = body.as_object_mut() else {
            return;
        };
        let rejected = self.rejected.load(Ordering::Relaxed);
        if rejected & COMPLETION_TOKENS != 0
            && let Some(tokens) = fields.get("max_tokens").and_then(Value::as_u64)
        {
            fields.remove("max_tokens");
            fields.insert(
                "max_completion_tokens".to_owned(),
                tokens.saturating_add(REASONING_TOKENS).into(),
            );
        }
        for (field, flag) in &PARAMETERS[1..] {
            if rejected & flag != 0 {
                fields.remove(*field);
            }
        }
    }

    pub fn reject(&self, error: &OpenAIError, body: &mut Value) -> bool {
        let OpenAIError::ApiError(response) = error else {
            return false;
        };
        if !matches!(
            response.status_code,
            StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY
        ) {
            return false;
        }
        let error = &response.api_error;
        let message = error.message.to_lowercase();
        let unsupported = matches!(
            error.code.as_deref(),
            Some("unsupported_parameter" | "unknown_parameter" | "unsupported_value")
        ) || [
            "unsupported",
            "not supported",
            "does not support",
            "unknown parameter",
            "unrecognized parameter",
            "not allowed",
            "not permitted",
            "invalid option",
            "不支持",
            "未知参数",
        ]
        .iter()
        .any(|phrase| message.contains(phrase));
        if !unsupported {
            return false;
        }
        let parameter = if let Some(parameter) = error.param.as_deref() {
            // thinking.type 这类子字段被拒时，去掉整个可选对象。
            PARAMETERS
                .iter()
                .find(|(field, _)| parameter.split('.').next() == Some(*field))
        } else {
            let mut mentioned = PARAMETERS.iter().filter(|(field, _)| {
                body.get(*field).is_some()
                    && message
                        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                        .any(|word| word == *field)
            });
            let first = mentioned.next();
            // 没有结构化字段时，只在指向唯一可选参数的情况下重试。
            if mentioned.next().is_some() {
                return false;
            }
            first
        };
        let Some((parameter, flag)) = parameter else {
            return false;
        };
        if *parameter == "max_tokens" {
            let plain = message.replace(['\'', '"', '`'], "");
            let field_rejected = matches!(
                error.code.as_deref(),
                Some("unsupported_parameter" | "unknown_parameter")
            ) || [
                "unsupported parameter",
                "unknown parameter",
                "unrecognized parameter",
                "max_tokens is not supported",
                "max_tokens is unsupported",
                "max_completion_tokens",
            ]
            .iter()
            .any(|phrase| plain.contains(phrase));
            if !field_rejected {
                return false;
            }
        }
        if body.get(*parameter).is_none() {
            return false;
        }
        self.rejected.fetch_or(*flag, Ordering::Relaxed);
        self.apply(body);
        true
    }

    pub fn disable_thinking(&self) {
        self.rejected
            .fetch_or(REASONING_EFFORT | THINKING, Ordering::Relaxed);
    }
}
