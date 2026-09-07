use serde::{Deserialize, Serialize};

use super::InputSource;

/// 一次上屏。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitEntry {
    /// 本进程内单调递增的序号，撤销条目用它指回来。
    pub id: u64,

    /// 查询时整段作用域的原始键（回放评测就是把它重新喂给引擎）；双拼下是双拼键，纠错时是敲错的原串。
    #[serde(default)]
    pub scope: String,

    /// 这次上屏消耗掉的那部分原始键（`scope` 的前缀）。
    pub keys: String,

    /// 查询时整段作用域的切分（`'` 连接；纠错生效时是纠正后的拼音）。
    pub pinyin: String,

    /// 拼写纠错是否生效。
    pub corrected: bool,

    /// 上屏的文字。
    pub text: String,

    /// 来源。
    pub source: InputSource,

    /// 选的是查询时候选列表里的第几个（从 0 数）；不在列表里（晚到的云端词、原样上屏）为 `None`。
    pub index: Option<usize>,

    /// 查询时排在前面的几个候选文本，用来离线算首选命中率。
    pub top: Vec<String>,

    /// 双拼方案的键（`xiaohe`），全拼为空。
    pub scheme: String,

    /// 是否在英文模式。
    pub english: bool,
}

/// 日志里的一条。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum InputLogEntry {
    /// 上屏。
    Commit(CommitEntry),

    /// 用户把上一次上屏的词整个退格删掉又对同一段拼音选了别的：那次选错了。
    Retract {
        /// 被撤销的那条上屏的 `id`。
        of: u64,

        /// 被撤销的文字。
        text: String,

        /// 改选的文字。
        chosen: String,
    },
}
