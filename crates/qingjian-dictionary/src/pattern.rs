/// 把键盘输入常用的 `lue` / `nue` 转成词库规范形式。
///
/// 词库用 `v` 表示 ü。j / q / x / y 后 ü 的正字法写作 `u`，但键盘上 `jv` / `qv` / `xv` / `yv`
/// 是常见输入习惯（与微软、搜狗一致），也归一到规范写法。其他音节原样返回。
pub fn canonical_syllable(text: &str) -> &str {
    match text {
        "lue" => "lve",
        "nue" => "nve",
        "jv" => "ju",
        "qv" => "qu",
        "xv" => "xu",
        "yv" => "yu",
        _ => text,
    }
}

/// 一个音节的查询条件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyllablePattern<'a> {
    /// 用户敲的字母：完整音节，或音节前缀 / 声母。
    pub text: &'a str,

    /// 为真时要求音节与 `text` 完全相等，否则只要求以 `text` 开头（简拼、未打完的音节）。
    pub complete: bool,
}

impl<'a> SyllablePattern<'a> {
    pub fn complete(text: &'a str) -> Self {
        Self {
            text,
            complete: true,
        }
    }

    pub fn prefix(text: &'a str) -> Self {
        Self {
            text,
            complete: false,
        }
    }

    pub fn accepts(&self, syllable: &str) -> bool {
        let text = canonical_syllable(self.text);
        let syllable = canonical_syllable(syllable);
        if self.complete {
            syllable == text
        } else {
            syllable.starts_with(text)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_keyboard_spelling_for_u_umlaut() {
        assert_eq!(canonical_syllable("lue"), "lve");
        assert_eq!(canonical_syllable("nue"), "nve");
        assert_eq!(canonical_syllable("lve"), "lve");
        assert_eq!(canonical_syllable("nve"), "nve");
        assert_eq!(canonical_syllable("xue"), "xue");
        assert_eq!(canonical_syllable("jv"), "ju");
        assert_eq!(canonical_syllable("qv"), "qu");
        assert_eq!(canonical_syllable("xv"), "xu");
        assert_eq!(canonical_syllable("yv"), "yu");
    }

    #[test]
    fn accepts_keyboard_and_canonical_spellings_equally() {
        assert!(SyllablePattern::complete("lue").accepts("lve"));
        assert!(SyllablePattern::complete("nve").accepts("nue"));
        assert!(!SyllablePattern::complete("xue").accepts("xve"));
    }
}
