use serde::{Deserialize, Serialize};

/// 候选的来源类型，平台层可据此区别显示。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CandidateKind {
    /// 中文词库里的词。
    Chinese,

    /// 形码码表里的字词（五笔）。编码平铺，没有音节：`syllables` 为空，上屏吃掉整段编码。
    Code,

    /// 英文词表里的词（中英混输），上屏时吃掉整段输入。
    English,

    /// 云联想给出的词：带全拼音节，上屏后记成用户词。
    Cloud,

    /// 快捷候选（日期 / 时间 / 星期 / 算式结果 / 中文数字），由输入直接算出，上屏时吃掉整段作用域、不记学习。
    Shortcut,

    /// 自定义短语，带它在列表里的固定位置（从 1 开始，按权重排出来的名次）。
    Custom(usize),

    /// 按候选词配的 emoji，音节与那个词相同；上屏按音节消耗拼音，不记学习。
    Emoji,

    /// 按候选词配的颜文字，规则同 `Emoji`。
    Kaomoji,

    /// 按候选词配的符号（约等于 → ≈），规则同 `Emoji`。
    Symbol,

    /// 离线整句转换的结果（多个词拼成），带全部音节；上屏按音节消耗拼音，路径上的词逐条记入个人 n-gram，不记词频。
    Sentence,

    /// 模型直接按按键生成的整句（`yongdockerbushuhenfangbian` → 用docker部署很方便）：
    /// 词图读不通整段输入时的兜底来源，没有音节对齐，上屏吃掉整段作用域、不记学习。
    Generated,
}

impl CandidateKind {
    /// 跟在某个词后面配出来的（emoji、颜文字、符号）：音节与那个词相同，`reading` 是那个词。
    pub fn follows_word(self) -> bool {
        matches!(self, Self::Emoji | Self::Kaomoji | Self::Symbol)
    }

    /// 删得掉的（[`crate::Engine::forget`]）：中文词、码表词、云端词删用户词与学习记录，英文词删个人英文词；
    /// 整句、快捷候选、自定义短语、emoji 这些没什么可删，壳不必给删除入口。
    pub fn forgettable(self) -> bool {
        match self {
            Self::Chinese | Self::Code | Self::Cloud | Self::English => true,
            Self::Shortcut
            | Self::Custom(_)
            | Self::Emoji
            | Self::Kaomoji
            | Self::Symbol
            | Self::Sentence
            | Self::Generated => false,
        }
    }
}
