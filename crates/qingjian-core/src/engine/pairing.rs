//! 成对符号（#928）的引擎入口：壳在一个符号要上屏前问 [`Engine::pair_punctuation`]，没在组句时按退格问 [`Engine::pair_backspace`]。
//! 规则在 [`crate::punctuation::Pairs`]。

use super::Engine;
use crate::punctuation::PairEdit;

impl Engine {
    /// 成对符号开关：`chinese` 管中文与日文模式（全角括号、引号、书名号），`english` 管英文模式（半角括号与双引号）。
    pub fn set_pair_punctuation(&mut self, chinese: bool, english: bool) {
        self.pair_punctuation = (chinese, english);
    }

    /// 符号 `text` 要上屏了（中文模式是转好的全角标点，英文模式是原字符）；`english` 是当前在英文模式。
    /// 返回 `None` 照常上屏，否则按 [`PairEdit`] 改文档。补上的右半边不管开关，敲到时都跳过。
    pub fn pair_punctuation(&mut self, text: &str, english: bool) -> Option<PairEdit> {
        let enabled = if english {
            self.pair_punctuation.1
        } else {
            self.pair_punctuation.0
        };
        self.punctuation.pair(text, enabled)
    }

    /// 没在组句时按了退格：刚补完一对、中间什么都没打时返回这一对，壳核对光标前后正是它们就一起删，
    /// 对不上就照常退格。
    pub fn pair_backspace(&mut self) -> Option<(char, char)> {
        self.punctuation.pairs().backspace()
    }
}
