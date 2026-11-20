//! 标点转换、半角符号与自定义短语。

use crate::engine::Engine;

impl Engine {
    /// 设置中文模式的标点转换。
    pub fn set_full_width_punctuation(&mut self, enabled: bool) {
        self.full_width_punctuation = enabled;
    }

    /// 中文标点开着也保持半角的符号（写代码常用的 `` ` `` 之类），只认会转全角的标点（[`crate::punctuation::converts`]）。
    pub fn set_half_width_symbols(&mut self, symbols: &str) {
        self.half_width_symbols = symbols
            .chars()
            .filter(|c| crate::punctuation::converts(*c))
            .collect();
    }

    /// 原子更新自定义短语，非法规则保持旧值。
    pub fn set_custom_phrases(&mut self, phrases: Vec<crate::CustomPhrase>) -> Result<(), String> {
        crate::custom_phrase::validate_phrases(&phrases)?;
        self.custom_phrases = phrases;
        Ok(())
    }
}
