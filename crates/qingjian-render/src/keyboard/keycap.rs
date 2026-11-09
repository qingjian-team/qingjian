//! 一个键：左上键名，右上角标，下方几行小字。

use super::{KeycapState, Root};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Keycap {
    /// 键名（`Q`、`;`）。
    pub label: String,

    /// 右上角标（翘舌声母）。
    pub corner: Option<String>,

    /// 下方的小字，一项一行（韵母）；[`KeycapState::Next`] 时改画那个音节。
    pub lines: Vec<String>,

    pub state: KeycapState,

    /// 字根（五笔）：有它时键面改成字根网格，键名与 [`Self::corner`]（区位号）挪到底下一行，不画 [`Self::lines`]。
    pub roots: Vec<Root>,
}
