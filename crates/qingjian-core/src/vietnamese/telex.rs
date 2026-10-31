//! Simple Telex 组词器。原始按键是唯一状态，每次编辑后重算，方便退格与恢复。

use vi::methods::transform_buffer_incremental_with_style;
use vi::processor::AccentStyle;

/// 一个尚未上屏的越南语或英文单词。
#[derive(Debug, Clone, Default)]
pub struct TelexComposer {
    /// 已确认但尚未按 Enter 上屏的整段前缀（含词间空格 / 标点）。
    committed: String,

    raw: String,

    /// 当前正在编辑的一个词。
    current: String,

    /// `committed + current`，直接作为平台 marked text / composition。
    preedit: String,

    restored: bool,
}

impl TelexComposer {
    /// 收一个 ASCII 字母。其他字符是词边界，由平台层先 [`Self::commit`] 再处理。
    pub fn push(&mut self, c: char) -> bool {
        if !c.is_ascii_alphabetic() {
            return false;
        }
        self.raw.push(c);
        self.recompute();
        true
    }

    /// 删除一枚原始按键，并按剩余按键重算。
    pub fn backspace(&mut self) -> bool {
        if self.raw.pop().is_some() {
            self.restored = false;
            self.recompute();
            return true;
        }
        let removed = self.committed.pop().is_some();
        self.rebuild_preedit();
        removed
    }

    /// 把本词恢复成用户敲的原始按键；用于 Esc。
    pub fn restore(&mut self) -> bool {
        if self.raw.is_empty() || self.current == self.raw {
            return false;
        }
        self.restored = true;
        self.current.clone_from(&self.raw);
        self.rebuild_preedit();
        true
    }

    /// 当前给系统 composition / marked text 显示的文字。
    pub fn preedit(&self) -> &str {
        &self.preedit
    }

    /// 当前词的原始 Telex 按键。
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// 当前词的 Telex 转换结果；候选词库只按这一段查，不含前面已经确认的整段。
    pub fn current(&self) -> &str {
        &self.current
    }

    /// 当前整段连续前缀（已确认前缀 + 当前词），用于优先查完整越南语短语。
    pub fn phrase_prefix(&self) -> &str {
        &self.preedit
    }

    pub fn is_empty(&self) -> bool {
        self.raw.is_empty() && self.committed.is_empty()
    }

    /// 确认当前词并把边界字符留在尚未上屏的整段中。
    pub fn commit_word(&mut self, text: Option<&str>, boundary: char) -> bool {
        let word = text.unwrap_or(&self.current);
        if word.is_empty() {
            return false;
        }
        self.committed.push_str(word);
        self.committed.push(boundary);
        self.raw.clear();
        self.current.clear();
        self.restored = false;
        self.rebuild_preedit();
        true
    }

    /// 选择以整段为前缀查出的完整短语：它替换整段，而不是追加到已确认前缀后。
    pub fn choose_phrase(&mut self, phrase: &str, boundary: char) -> bool {
        if phrase.is_empty() {
            return false;
        }
        self.committed.clear();
        self.committed.push_str(phrase);
        self.committed.push(boundary);
        self.raw.clear();
        self.current.clear();
        self.restored = false;
        self.rebuild_preedit();
        true
    }

    /// 取出整段显示文字并结束本轮 composition（只在 Enter / 失焦时调用）。
    pub fn commit(&mut self) -> String {
        let text = std::mem::take(&mut self.preedit);
        self.committed.clear();
        self.raw.clear();
        self.current.clear();
        self.restored = false;
        text
    }

    pub fn clear(&mut self) {
        self.committed.clear();
        self.raw.clear();
        self.current.clear();
        self.preedit.clear();
        self.restored = false;
    }

    fn recompute(&mut self) {
        if self.raw.is_empty() {
            self.current.clear();
            self.rebuild_preedit();
            return;
        }
        if self.restored || starts_with_w(&self.raw) {
            self.current.clone_from(&self.raw);
            self.rebuild_preedit();
            return;
        }
        let mut buffer = transform_buffer_incremental_with_style(&vi::TELEX, AccentStyle::New);
        for c in self.raw.chars() {
            buffer.push(c);
        }
        self.current.clear();
        self.current.push_str(buffer.view());
        self.collapse_literal_modifier_pair();
        self.rebuild_preedit();
    }

    /// UniKey 的重复键恢复也适用于第一次没能当声调处理的键：`guitarr` 得 `guitar`。
    fn collapse_literal_modifier_pair(&mut self) {
        let mut raw = self.raw.chars().rev();
        let Some(last) = raw.next() else {
            return;
        };
        if raw.next() != Some(last)
            || !matches!(last.to_ascii_lowercase(), 's' | 'f' | 'r' | 'x' | 'j')
        {
            return;
        }
        let mut output = self.current.chars().rev();
        if output.next() == Some(last) && output.next() == Some(last) {
            self.current.pop();
        }
    }

    fn rebuild_preedit(&mut self) {
        self.preedit.clear();
        self.preedit.push_str(&self.committed);
        self.preedit.push_str(&self.current);
    }
}

fn starts_with_w(text: &str) -> bool {
    text.as_bytes()
        .first()
        .is_some_and(|byte| byte.eq_ignore_ascii_case(&b'w'))
}

#[cfg(test)]
mod tests {
    use super::TelexComposer;

    fn compose(keys: &str) -> String {
        let mut composer = TelexComposer::default();
        assert!(keys.chars().all(|c| composer.push(c)));
        composer.preedit().to_owned()
    }

    #[test]
    fn composes_simple_telex_with_modern_tone_placement() {
        for (keys, expected) in [
            ("tieengs", "tiếng"),
            ("Vieetj", "Việt"),
            ("dduwowngf", "đường"),
            ("vuwofwn", "vườn"),
            ("hoangf", "hoàng"),
            ("owf", "ờ"),
            ("ofw", "ờ"),
            ("asf", "à"),
            ("asz", "a"),
            ("toanfs", "toán"),
        ] {
            assert_eq!(compose(keys), expected, "{keys}");
        }
    }

    #[test]
    fn keeps_non_vietnamese_words_but_preserves_unikey_ambiguities() {
        for word in ["linux", "changes", "windows", "request", "keyboard"] {
            assert_eq!(compose(word), word, "{word}");
        }
        assert_eq!(compose("this"), "thí");
        assert_eq!(compose("mix"), "mĩ");
    }

    #[test]
    fn repeated_modifiers_restore_literal_text() {
        assert_eq!(compose("herr"), "her");
        assert_eq!(compose("xooong"), "xoong");
        assert_eq!(compose("guitarr"), "guitar");
    }

    #[test]
    fn simple_telex_does_not_turn_initial_w_into_u_horn() {
        assert_eq!(compose("w"), "w");
        assert_eq!(compose("windows"), "windows");
        assert_eq!(compose("uw"), "ư");
    }

    #[test]
    fn editing_replays_raw_keys_and_restore_is_per_word() {
        let mut composer = TelexComposer::default();
        for c in "tieengs".chars() {
            composer.push(c);
        }
        assert_eq!(composer.preedit(), "tiếng");
        assert!(composer.backspace());
        assert_eq!(composer.preedit(), "tiêng");
        composer.push('s');
        assert!(composer.restore());
        assert_eq!(composer.preedit(), "tieengs");
        assert!(!composer.restore());
        assert_eq!(composer.commit(), "tieengs");
        assert!(composer.is_empty());
        composer.push('a');
        composer.push('s');
        assert_eq!(composer.preedit(), "á");
    }

    #[test]
    fn rejects_non_letters_and_clear_discards_the_word() {
        let mut composer = TelexComposer::default();
        assert!(!composer.push('1'));
        assert!(composer.push('a'));
        composer.clear();
        assert!(composer.is_empty());
        assert_eq!(composer.preedit(), "");
    }

    #[test]
    fn space_keeps_a_sentence_composing_until_enter_commit() {
        let mut composer = TelexComposer::default();
        for c in "tieengs".chars() {
            composer.push(c);
        }
        assert!(composer.commit_word(None, ' '));
        assert_eq!(composer.preedit(), "tiếng ");
        assert!(composer.raw().is_empty());

        for c in "Vieetj".chars() {
            composer.push(c);
        }
        assert_eq!(composer.preedit(), "tiếng Việt");
        assert_eq!(composer.commit(), "tiếng Việt");
        assert!(composer.is_empty());
    }

    #[test]
    fn backspace_can_edit_the_confirmed_sentence_prefix() {
        let mut composer = TelexComposer::default();
        for c in "xin".chars() {
            composer.push(c);
        }
        assert!(composer.commit_word(None, ' '));
        assert!(composer.backspace());
        assert_eq!(composer.preedit(), "xin");
    }
}
