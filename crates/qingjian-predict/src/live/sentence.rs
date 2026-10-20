//! 跟踪实际上屏的历史与暂定候选，退格后重新计算句界。

#[derive(Debug, Default)]
pub struct SentenceTracker {
    committed: String,

    preview: String,
}

impl SentenceTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn commit(&mut self, text: &str) {
        self.preview.clear();
        self.committed.push_str(text);
        // 限制长时间输入会话的内存，保留足够的跨句退格上下文。
        if self.committed.len() > 32768 {
            let mut split = self.committed.len() - 16384;
            while !self.committed.is_char_boundary(split) {
                split += 1;
            }
            self.committed.drain(..split);
        }
    }

    pub fn preview(&mut self, text: &str) {
        self.preview = text.to_owned();
    }

    pub fn backspace(&mut self) {
        self.preview.clear();
        self.committed.pop();
    }

    pub fn reset(&mut self) {
        self.committed.clear();
        self.preview.clear();
    }

    /// 由平台确认的光标前文本替换历史；不包含当前未上屏候选。
    pub fn set_context(&mut self, before: &str) {
        self.reset();
        self.commit(before);
    }

    pub fn committed(&self) -> &str {
        &self.committed
    }

    pub fn reconcile_before(&mut self, before: Option<&str>) -> bool {
        if before.is_some_and(|text| {
            if text.is_empty() && !self.committed.is_empty() {
                return true;
            }
            // 编辑器往往只给有限的光标前上下文，只核验双方都可见的尾部。
            if text.len() < self.committed.len() {
                !self.committed.ends_with(text)
            } else {
                !text.ends_with(&self.committed)
            }
        }) {
            self.reset();
            return false;
        }
        true
    }

    pub fn source(&self) -> String {
        let text = format!("{}{}", self.committed, self.preview);
        current_sentence(&text).trim().to_owned()
    }
}

fn current_sentence(text: &str) -> &str {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut start = 0;
    let mut sentence_start = 0;
    let mut boundary = false;
    for (index, &(offset, ch)) in chars.iter().enumerate() {
        if boundary && !is_closer(ch) && !ch.is_whitespace() && !is_terminal(ch) {
            start = offset;
            boundary = false;
        }
        if is_terminal(ch) && (ch != '.' || period_ends(text, &chars, index)) {
            sentence_start = start;
            boundary = true;
        }
    }
    // 刚输入句末标点时仍展示这一句，下一句有内容后才移动起点。
    &text[if boundary { sentence_start } else { start }..]
}

fn is_terminal(ch: char) -> bool {
    matches!(ch, '。' | '.' | '?' | '？' | '!' | '！' | '\n' | '\r')
}

fn is_closer(ch: char) -> bool {
    matches!(
        ch,
        '”' | '’' | '"' | '\'' | ')' | '）' | ']' | '】' | '}' | '》' | '」' | '』'
    )
}

fn period_ends(text: &str, chars: &[(usize, char)], index: usize) -> bool {
    let prev = index.checked_sub(1).map(|i| chars[i].1);
    let next = chars.get(index + 1).map(|(_, ch)| *ch);
    if prev == Some('.')
        || next == Some('.')
        || (prev.is_some_and(|ch| ch.is_ascii_digit())
            && (next.is_none() || next.is_some_and(|ch| ch.is_ascii_digit())))
    {
        return false;
    }
    let offset = chars[index].0;
    let token = text[..offset].split_whitespace().next_back().unwrap_or("");
    if (token.contains('@') || token.contains("://") || token.starts_with("www."))
        && next.is_some_and(|ch| !ch.is_whitespace() && !is_closer(ch))
    {
        return false;
    }
    let word = token
        .trim_start_matches(|ch: char| !ch.is_ascii_alphabetic())
        .to_ascii_lowercase();
    if matches!(
        word.as_str(),
        "mr" | "mrs" | "ms" | "dr" | "prof" | "sr" | "jr" | "st" | "vs" | "etc" | "e.g" | "i.e"
    ) {
        return false;
    }
    // 英文缩写/首字母（U.S.、A. Smith）不作为新句起点。
    if (word.len() == 1 && word != "i")
        || (word.contains('.')
            && word
                .split('.')
                .all(|part| part.len() == 1 && part.bytes().all(|ch| ch.is_ascii_alphabetic())))
        || next.is_some_and(|ch| ch.is_ascii_alphanumeric())
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::SentenceTracker;

    #[test]
    fn preview_is_replaced_and_commit_does_not_duplicate() {
        let mut tracker = SentenceTracker::new();
        tracker.commit("我正在");
        tracker.preview("学习");
        assert_eq!(tracker.source(), "我正在学习");
        tracker.preview("学英语");
        assert_eq!(tracker.source(), "我正在学英语");
        tracker.commit("学英语");
        assert_eq!(tracker.source(), "我正在学英语");
    }

    #[test]
    fn boundaries_and_backspace_restore_sentence() {
        let mut tracker = SentenceTracker::new();
        tracker.commit("你好，世界；欢迎：你。\"下一句？！\"");
        assert_eq!(tracker.source(), "下一句？！\"");
        tracker.backspace();
        tracker.backspace();
        tracker.backspace();
        assert_eq!(tracker.source(), "下一句");
        tracker.reset();
        tracker.commit("你好。");
        assert_eq!(tracker.source(), "你好。");
        tracker.backspace();
        tracker.commit("世界");
        assert_eq!(tracker.source(), "你好世界");
    }

    #[test]
    fn exceptions_and_newlines() {
        for text in [
            "Dr. Smith来了",
            "价格3.14元",
            "访问https://example.com/a",
            "写信a@b.com",
            "等一下...然后继续",
            "等一下……然后继续",
            "U.S. economy",
        ] {
            let mut tracker = SentenceTracker::new();
            tracker.commit(text);
            assert_eq!(tracker.source(), text);
        }
        let mut tracker = SentenceTracker::new();
        tracker.commit("第一句\n第二句");
        assert_eq!(tracker.source(), "第二句");
    }

    #[test]
    fn numeric_and_url_sentence_endings_are_reconsidered() {
        for text in [
            "I am 18. Next sentence",
            "I. Next",
            "See https://example.com. Next",
            "Email a@b.com. Next",
            "See example.com. Next sentence",
        ] {
            let mut tracker = SentenceTracker::new();
            tracker.commit(text);
            assert!(
                tracker.source().starts_with("Next"),
                "{text}: {}",
                tracker.source()
            );
        }
        let mut tracker = SentenceTracker::new();
        tracker.commit("价格3.");
        tracker.commit("14元");
        assert_eq!(tracker.source(), "价格3.14元");
    }

    #[test]
    fn context_mismatch_discards_history_without_importing_untyped_text() {
        let mut tracker = SentenceTracker::new();
        tracker.commit("你好");
        assert!(tracker.reconcile_before(Some("既有文档你好")));
        assert!(tracker.reconcile_before(None));
        assert!(!tracker.reconcile_before(Some("另一个输入框")));
        assert_eq!(tracker.source(), "");
        tracker.commit("非空历史");
        assert!(tracker.reconcile_before(Some("历史")));
        assert!(!tracker.reconcile_before(Some("")));
        assert_eq!(tracker.source(), "");
    }
}
