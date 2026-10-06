use serde::{Deserialize, Serialize};

use crate::candidate::CandidateKind;

/// 上屏的文字从哪来。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputSource {
    /// 原生 Rime 上屏，原生翻译器决定候选类型与造词。
    Rime,

    /// 词库里的词（含用户词）。
    Word,

    /// 云端词。
    Cloud,

    /// 本地整句转换。
    Sentence,

    /// 英文候选。
    English,

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
            CandidateKind::Shortcut => Self::Shortcut,
            CandidateKind::Custom(_) => Self::Custom,
            CandidateKind::Emoji => Self::Emoji,
        }
    }
}
