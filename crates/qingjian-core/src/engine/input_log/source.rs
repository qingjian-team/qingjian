use serde::{Deserialize, Serialize};

use crate::candidate::CandidateKind;

/// 上屏的文字从哪来。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputSource {
    /// 词库里的词（含用户词）。
    Word,

    /// 云端词。
    Cloud,

    /// 本地整句转换。
    Sentence,

    /// 英文候选。
    English,

    /// 越南语词候选。
    VietnameseWord,

    /// 越南语短语或多词组合候选。
    VietnamesePhrase,

    /// 越南语模式下原始 ASCII 兜底。
    VietnameseRaw,

    /// 快捷候选（日期 / 算式 / 码点）。
    Shortcut,

    /// 用户配置的自定义短语。
    Custom,

    /// emoji。
    Emoji,

    /// 回车原样上屏敲的字母。
    Raw,

    /// 上屏的是候选的译词（修饰键 + 数字）。
    Translation,

    /// Tab 接受的云端整句补全。
    CloudSentence,
}

impl From<CandidateKind> for InputSource {
    fn from(kind: CandidateKind) -> Self {
        match kind {
            // 形码的词也是词库里的词，输入日志的来源不另分（方案记在别的字段）
            CandidateKind::Chinese | CandidateKind::Code => Self::Word,
            CandidateKind::Cloud => Self::Cloud,
            // 模型直接生成的也是整句，回放时与词图出的整句同一类看
            CandidateKind::Sentence | CandidateKind::Generated => Self::Sentence,
            CandidateKind::English => Self::English,
            CandidateKind::VietnameseWord => Self::VietnameseWord,
            CandidateKind::VietnamesePhrase => Self::VietnamesePhrase,
            CandidateKind::VietnameseRaw => Self::VietnameseRaw,
            CandidateKind::Shortcut => Self::Shortcut,
            CandidateKind::Custom(_) => Self::Custom,
            // 颜文字、符号与 emoji 同是跟在词后面配出来的，回放时一类看
            CandidateKind::Emoji | CandidateKind::Kaomoji | CandidateKind::Symbol => Self::Emoji,
        }
    }
}
