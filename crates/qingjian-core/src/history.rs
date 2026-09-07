//! 本地输入历史：本次会话里经我们上屏的文本，按时间顺序拼成一段。
//!
//! 只用于两件事：应用不提供光标附近文本时给联想当上下文；以后给个人模型当训练数据。
//! 全部在内存里，落盘由壳决定；容量有上限，超过就丢最旧的。

/// 内存里最多保留多少个字符。够联想的观察窗口用，也不会无限增长。
const DEFAULT_CAPACITY: usize = 4096;

#[derive(Debug, Clone)]
pub struct InputHistory {
    /// 已上屏文本，按时间顺序首尾相接。
    text: String,

    /// 字符数上限。
    capacity: usize,
}

impl Default for InputHistory {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }
}

impl InputHistory {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            text: String::new(),
            capacity,
        }
    }

    /// 追加一段上屏文本，超出容量时从头丢弃。
    pub fn record(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.text.push_str(text);
        let count = self.text.chars().count();
        if count > self.capacity {
            let drop = count - self.capacity;
            let start = self
                .text
                .char_indices()
                .nth(drop)
                .map_or(self.text.len(), |(i, _)| i);
            self.text.drain(..start);
        }
    }

    /// 最近 `chars` 个字符。
    pub fn recent(&self, chars: usize) -> &str {
        if chars == 0 {
            return "";
        }
        let count = self.text.chars().count();
        if chars >= count {
            return &self.text;
        }
        let start = self
            .text
            .char_indices()
            .nth(count - chars)
            .map_or(0, |(i, _)| i);
        &self.text[start..]
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// 一键清除。
    pub fn clear(&mut self) {
        self.text.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_in_order_and_returns_recent_chars() {
        let mut history = InputHistory::default();
        history.record("我们");
        history.record("今天");
        history.record("");
        assert_eq!(history.text(), "我们今天");
        assert_eq!(history.recent(2), "今天");
        assert_eq!(history.recent(10), "我们今天");
        assert_eq!(history.recent(0), "");
    }

    #[test]
    fn drops_oldest_beyond_capacity_on_char_boundaries() {
        let mut history = InputHistory::with_capacity(3);
        history.record("开发");
        history.record("输入法");
        assert_eq!(history.text(), "输入法");
        history.record("a");
        assert_eq!(history.text(), "入法a");
        history.clear();
        assert!(history.is_empty());
    }
}
