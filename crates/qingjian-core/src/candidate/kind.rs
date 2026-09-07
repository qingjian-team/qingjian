use serde::{Deserialize, Serialize};

/// 候选的来源类型，平台层可据此区别显示。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CandidateKind {
    /// 中文词库里的词。
    Chinese,

    /// 英文词表里的词（中英混输），上屏时吃掉整段输入。
    English,

    /// 云联想给出的词：带全拼音节，上屏后记成用户词。
    Cloud,

    /// 快捷候选（日期 / 时间 / 星期 / 算式结果 / 中文数字），由输入直接算出，上屏时吃掉整段作用域、不记学习。
    Shortcut,

    /// 按候选词配的 emoji，音节与那个词相同；上屏按音节消耗拼音，不记学习。
    Emoji,

    /// 离线整句转换的结果（多个词拼成），带全部音节；上屏按音节消耗拼音，路径上的词逐条记入个人 n-gram，不记词频。
    Sentence,
}
