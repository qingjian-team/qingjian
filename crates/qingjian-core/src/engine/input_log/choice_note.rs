//! 输入日志里选词模型那一项。

use serde::{Deserialize, Serialize};

/// 查询时选词模型排过这页：上屏条目带上它，配合 `index` 就能算「被它改过首位的，用户选第一个的比例」。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChoiceNote {
    /// 挪到第一位的那个候选原来排第几（从 0 数）；0 是首位没变。
    pub from: usize,

    /// 给模型看的前文有多少个字。
    pub ctx: usize,

    /// 前文是应用里光标前的文字；`false` 是应用给不出、用的本会话最近上屏的字。
    pub app: bool,

    /// 这页打分花的毫秒数。
    pub ms: u64,
}
