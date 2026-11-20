//! Engine 的标点与成对符号状态。

use crate::punctuation::Punctuation;

/// 标点状态与开关：全角标点的配对状态、中文标点转换、半角白名单、成对符号。
pub(crate) struct EnginePunctuation {
    /// 全角标点与引号配对状态。
    pub(crate) punctuation: Punctuation,

    /// 中文标点转换开关。
    pub(crate) full_width_punctuation: bool,

    /// 中文标点开着也保持半角的符号（`[general] half_width_symbols`，#893）。
    pub(crate) half_width_symbols: Vec<char>,

    /// 成对符号开关：中文与日文模式一份、英文模式一份（见 `Engine::pair_punctuation`）。
    pub(crate) pair_punctuation: (bool, bool),
}

impl Default for EnginePunctuation {
    fn default() -> Self {
        Self {
            punctuation: Punctuation::default(),
            full_width_punctuation: true,
            half_width_symbols: Vec::new(),
            pair_punctuation: (true, false),
        }
    }
}
