//! 统一错误分类（增量层）。
//!
//! Core 现有错误类型分散在各模块的 thiserror 定义里；这里在不删除、不改名它们的前提下，
//! 归到有限的 [`ErrorKind`] 大类，并用 [`CoreError`] 包一层上下文。原始错误保留在
//! source 链上，消息与细节不丢。
//!
//! 映射关系：
//! - [`crate::candidate::UnknownLanguage`] → [`ErrorKind::InvalidInput`]
//! - [`crate::candidate::UnknownPartOfSpeech`] → [`ErrorKind::InvalidInput`]
//! - [`crate::parser::ParseError`]（含全部变体）→ [`ErrorKind::InvalidInput`]

use std::error::Error as StdError;
use std::fmt;

use crate::candidate::{UnknownLanguage, UnknownPartOfSpeech};
use crate::parser::ParseError;

/// 错误大类：只描述「调用方该怎么处理」，不替代原始错误的细节信息。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// 输入 / 配置值不被认识或不符合格式（语言代码、词性、拼音串等）。
    InvalidInput,

    /// 需要的能力没有接上（模型、学习方、翻译方等缺省为空操作）。
    Unavailable,

    /// Core 内部状态不一致，或出现不该发生的情况。
    Internal,
}

impl ErrorKind {
    /// 大类的简短中文标签，写进错误消息与诊断信息。
    pub fn label(self) -> &'static str {
        match self {
            ErrorKind::InvalidInput => "输入不被认识",
            ErrorKind::Unavailable => "能力未接上",
            ErrorKind::Internal => "内部错误",
        }
    }
}

/// 带分类与上下文的统一错误。
///
/// `source` 保留原始错误；[`CoreError::context`] 记录包装时补充的现场信息
/// （在哪个解析步骤、哪项配置上失败）。两者都可读取，不做吞错。
#[derive(Debug)]
pub struct CoreError {
    kind: ErrorKind,
    context: String,
    source: Option<Box<dyn StdError + Send + Sync>>,
}

impl CoreError {
    /// 不带原始错误构造：只给分类与上下文。
    pub fn new(kind: ErrorKind, context: impl Into<String>) -> Self {
        Self {
            kind,
            context: context.into(),
            source: None,
        }
    }

    /// 包一层原始错误：分类 + 上下文 + source 链。
    pub fn wrap<E>(kind: ErrorKind, context: impl Into<String>, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self {
            kind,
            context: context.into(),
            source: Some(Box::new(source)),
        }
    }

    /// 错误大类。
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// 包装时补充的上下文信息。
    pub fn context(&self) -> &str {
        &self.context
    }

    /// 原始错误（source 链的第一环）。
    pub fn source_error(&self) -> Option<&(dyn StdError + Send + Sync + 'static)> {
        self.source.as_deref()
    }
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}：{}", self.kind.label(), self.context)?;
        if let Some(source) = &self.source {
            write!(f, "（{source}）")?;
        }
        Ok(())
    }
}

impl StdError for CoreError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_deref().map(|source| source as &dyn StdError)
    }
}

impl From<UnknownLanguage> for CoreError {
    fn from(value: UnknownLanguage) -> Self {
        Self::wrap(ErrorKind::InvalidInput, "语言代码无法识别", value)
    }
}

impl From<UnknownPartOfSpeech> for CoreError {
    fn from(value: UnknownPartOfSpeech) -> Self {
        Self::wrap(ErrorKind::InvalidInput, "词性写法无法识别", value)
    }
}

impl From<ParseError> for CoreError {
    fn from(value: ParseError) -> Self {
        Self::wrap(ErrorKind::InvalidInput, "拼音串解析失败", value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 编译期断言：统一错误能跨线程传递，source 链不绑线程局部状态。
    const _: fn() = || {
        fn require_send_sync<T: Send + Sync>() {}
        require_send_sync::<CoreError>();
    };

    #[test]
    fn maps_each_existing_error_type_to_invalid_input() {
        let language: CoreError = UnknownLanguage("fr".to_owned()).into();
        assert_eq!(language.kind(), ErrorKind::InvalidInput);
        assert!(language.source_error().unwrap().to_string().contains("fr"));

        let part_of_speech: CoreError = UnknownPartOfSpeech("xyz".to_owned()).into();
        assert_eq!(part_of_speech.kind(), ErrorKind::InvalidInput);
        assert!(
            part_of_speech
                .source_error()
                .unwrap()
                .to_string()
                .contains("xyz")
        );

        let parse: CoreError = ParseError::NoSegmentation.into();
        assert_eq!(parse.kind(), ErrorKind::InvalidInput);
        assert_eq!(
            parse.source_error().unwrap().to_string(),
            ParseError::NoSegmentation.to_string()
        );
    }

    #[test]
    fn maps_each_parse_error_variant_and_keeps_details() {
        let variants = [
            ParseError::Empty,
            ParseError::InvalidChar {
                position: 3,
                ch: '#',
            },
            ParseError::NoSegmentation,
        ];

        for variant in variants {
            let message = variant.to_string();
            let error: CoreError = variant.into();
            assert_eq!(error.kind(), ErrorKind::InvalidInput);
            assert_eq!(error.source_error().unwrap().to_string(), message);
        }
    }

    #[test]
    fn wrapped_context_does_not_drop_original_message() {
        let original = UnknownLanguage("jp".to_owned());
        let error = CoreError::wrap(ErrorKind::InvalidInput, "偏好设置里的学习语言", original);

        // 上下文与原始消息都进了 Display，原始值还能从 source 链读到。
        assert!(error.context().contains("偏好设置"));
        assert!(error.to_string().contains("jp"));
        assert!(error.source_error().unwrap().to_string().contains("jp"));
    }

    /// 确定性伪随机用例：对一批随机生成的语言代码做映射，原始串在包装后不丢。
    #[test]
    fn random_codes_survive_mapping_round_trip() {
        let mut state: u64 = 0x9E37_79B7_7F4A_7C15;

        for _ in 0..200 {
            // 线性同余生成一串随机小写字母，长度 1～6。
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let length = 1 + (state >> 33) as usize % 6;
            let mut code = String::new();
            for _ in 0..length {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let letter = b'a' + ((state >> 33) % 26) as u8;
                code.push(letter as char);
            }

            let error: CoreError = UnknownLanguage(code.clone()).into();
            assert_eq!(error.kind(), ErrorKind::InvalidInput);
            assert!(!error.context().is_empty());
            assert!(error.source_error().unwrap().to_string().contains(&code));
            assert!(error.to_string().contains(&code));
        }
    }

    #[test]
    fn internal_and_unavailable_kinds_carry_context_without_source() {
        let missing = CoreError::new(ErrorKind::Unavailable, "还没接语言模型");
        assert_eq!(missing.kind(), ErrorKind::Unavailable);
        assert!(missing.source_error().is_none());
        assert!(missing.to_string().contains("还没接语言模型"));

        let broken = CoreError::new(ErrorKind::Internal, "词图格子缺词");
        assert_eq!(broken.kind(), ErrorKind::Internal);
        assert!(broken.source_error().is_none());
    }
}
