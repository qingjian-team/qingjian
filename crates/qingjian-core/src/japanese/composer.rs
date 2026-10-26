//! 日语罗马音的组句缓冲区。

use super::romaji::{to_hiragana, to_katakana};
use super::words;

/// 边敲边出的汉字词最多几个。
const PREDICTIONS: usize = 6;

/// 敲进来的罗马音；显示与候选按它现转（汉字词、平假名、片假名、原样罗马音）。汉字只查测试用的小词表（`words`）。
#[derive(Debug, Default, Clone)]
pub struct RomajiComposer {
    raw: String,
}

impl RomajiComposer {
    /// 只收字母、`-`（长音）与 `'`（`n'` 分隔）。
    pub fn push(&mut self, c: char) -> bool {
        if c.is_ascii_alphabetic() || matches!(c, '-' | '\'') {
            self.raw.push(c);
            true
        } else {
            false
        }
    }

    pub fn backspace(&mut self) {
        self.raw.pop();
    }

    pub fn clear(&mut self) {
        self.raw.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// 组句时显示的样子：转好的假名接着没拼完的罗马音（`かn`）。
    pub fn preedit(&self) -> String {
        let (kana, pending) = to_hiragana(&self.raw);
        kana + &pending
    }

    /// 上屏用的平假名：末尾单个 `n` 算「ん」。
    pub fn hiragana(&self) -> String {
        let (kana, pending) = to_hiragana(&self.raw);
        if pending == "n" {
            kana + "ん"
        } else {
            kana + &pending
        }
    }

    /// 候选：读音对得上的汉字词、平假名、片假名、以它开头的汉字词、原样罗马音，去重；没在敲时为空。
    pub fn candidates(&self) -> Vec<String> {
        if self.raw.is_empty() {
            return Vec::new();
        }
        let hiragana = self.hiragana();
        let (stem, pending) = to_hiragana(&self.raw);
        // 还在拼的罗马音（`niho`）不查完整读音，只按转好的那截往后猜
        let complete = pending.is_empty() || pending == "n";
        let mut out: Vec<String> = Vec::new();
        let mut add = |text: String| {
            if !text.is_empty() && !out.contains(&text) {
                out.push(text);
            }
        };
        if complete {
            words::exact(&hiragana)
                .into_iter()
                .for_each(|w| add(w.to_owned()));
        }
        add(hiragana.clone());
        add(to_katakana(&hiragana));
        words::starting_with(&stem, PREDICTIONS)
            .into_iter()
            .for_each(|w| add(w.to_owned()));
        add(self.raw.clone());
        out
    }
}
