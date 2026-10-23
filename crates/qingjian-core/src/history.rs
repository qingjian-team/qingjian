//! 本地输入历史：本次会话里经我们上屏的文本，按应用分开、按时间顺序拼成一段。
//!
//! 应用不提供光标前文时给模型当前文用，所以只认当前应用、当前输入框里最近还在打的那段：
//! 换了应用不串，换了输入框从头记，隔太久没打的当作过期。全部在内存里，容量有上限，超过就丢最旧的。

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// 每个应用最多保留多少个字符。前文只看最近几十字，长了没用。
const DEFAULT_CAPACITY: usize = 256;

/// 最多记几个应用，超过就丢最久没上屏的那个。
const MAX_APPLICATIONS: usize = 16;

/// 一个应用这么久没上屏，它的记录就不再当前文（[`InputHistory::set_fresh_for`] 可改）。
pub const DEFAULT_FRESH_SECONDS: u64 = 60;

#[derive(Debug, Clone)]
struct Entry {
    /// 已上屏文本，按时间顺序首尾相接。
    text: String,

    /// 最后一次上屏的时刻。
    last: Instant,
}

#[derive(Debug, Clone)]
pub struct InputHistory {
    /// 应用标识 → 它的记录；壳没报应用的记在空串下。
    entries: HashMap<String, Entry>,

    /// 当前应用（[`Self::set_application`]）。
    application: String,

    /// 每个应用的字符数上限。
    capacity: usize,

    /// 多久没上屏就过期；`None` 不过期。
    fresh_for: Option<Duration>,
}

impl Default for InputHistory {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }
}

impl InputHistory {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            application: String::new(),
            capacity,
            fresh_for: Some(Duration::from_secs(DEFAULT_FRESH_SECONDS)),
        }
    }

    /// 多少秒没上屏就不再当前文，0 表示一直算。
    pub fn set_fresh_for(&mut self, seconds: u64) {
        self.fresh_for = (seconds > 0).then(|| Duration::from_secs(seconds));
    }

    pub(crate) fn fresh_like(&mut self, other: &InputHistory) {
        self.fresh_for = other.fresh_for;
    }

    fn stale(&self, entry: &Entry, now: Instant) -> bool {
        self.fresh_for
            .is_some_and(|fresh| now.duration_since(entry.last) > fresh)
    }

    /// 切到另一个应用：之后的记录与读取都只看它的。
    pub fn set_application(&mut self, app: Option<&str>) {
        self.application = app.unwrap_or_default().to_owned();
    }

    /// 给当前应用追加一段上屏文本，超出容量时从头丢弃。
    pub fn record(&mut self, text: &str) {
        self.record_at(text, Instant::now());
    }

    fn record_at(&mut self, text: &str, now: Instant) {
        if text.is_empty() {
            return;
        }
        if !self.entries.contains_key(&self.application)
            && self.entries.len() >= MAX_APPLICATIONS
            && let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.last)
                .map(|(app, _)| app.clone())
        {
            self.entries.remove(&oldest);
        }
        let stale = self
            .entries
            .get(&self.application)
            .is_some_and(|entry| self.stale(entry, now));
        let entry = self
            .entries
            .entry(self.application.clone())
            .or_insert_with(|| Entry {
                text: String::new(),
                last: now,
            });
        // 过期的旧文本不和新打的接在一起
        if stale {
            entry.text.clear();
        }
        entry.last = now;
        entry.text.push_str(text);
        let count = entry.text.chars().count();
        if count > self.capacity {
            let drop = count - self.capacity;
            let start = entry
                .text
                .char_indices()
                .nth(drop)
                .map_or(entry.text.len(), |(i, _)| i);
            entry.text.drain(..start);
        }
    }

    /// 当前应用最近 `chars` 个字符；过期了是空串。
    pub fn recent(&self, chars: usize) -> &str {
        self.recent_at(chars, Instant::now())
    }

    fn recent_at(&self, chars: usize, now: Instant) -> &str {
        let Some(entry) = self.entries.get(&self.application) else {
            return "";
        };
        if chars == 0 || self.stale(entry, now) {
            return "";
        }
        let count = entry.text.chars().count();
        if chars >= count {
            return &entry.text;
        }
        let start = entry
            .text
            .char_indices()
            .nth(count - chars)
            .map_or(0, |(i, _)| i);
        &entry.text[start..]
    }

    /// 当前应用记下的全部文本（不管过没过期）。
    pub fn text(&self) -> &str {
        self.entries
            .get(&self.application)
            .map_or("", |entry| entry.text.as_str())
    }

    pub fn is_empty(&self) -> bool {
        self.entries.values().all(|entry| entry.text.is_empty())
    }

    /// 焦点进了另一个输入框：当前应用从头记。
    pub fn forget_current(&mut self) {
        self.entries.remove(&self.application);
    }

    /// 一键清除所有应用的记录。
    pub fn clear(&mut self) {
        self.entries.clear();
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

    #[test]
    fn each_application_keeps_its_own_text() {
        let mut history = InputHistory::default();
        history.set_application(Some("com.apple.Notes"));
        history.record("会议纪要");
        history.set_application(Some("com.tencent.xinWeChat"));
        assert_eq!(history.recent(10), "");
        history.record("晚上吃啥");
        history.set_application(Some("com.apple.Notes"));
        assert_eq!(history.recent(10), "会议纪要");
        history.set_application(None);
        assert_eq!(history.recent(10), "");
    }

    #[test]
    fn stale_text_is_not_context() {
        let mut history = InputHistory::default();
        let start = Instant::now();
        history.record_at("很久以前", start);
        let later = start + Duration::from_secs(DEFAULT_FRESH_SECONDS + 1);
        assert_eq!(history.recent_at(10, start), "很久以前");
        assert_eq!(history.recent_at(10, later), "");
        history.record_at("刚才", later);
        assert_eq!(history.recent_at(10, later), "刚才");
        history.set_fresh_for(0);
        history.record_at("一直算", start);
        assert_eq!(
            history.recent_at(10, later + later.duration_since(start)),
            "刚才一直算"
        );
    }

    #[test]
    fn a_new_input_box_starts_afresh() {
        let mut history = InputHistory::default();
        history.set_application(Some("com.apple.Notes"));
        history.record("会议纪要");
        history.set_application(Some("com.tencent.xinWeChat"));
        history.record("晚上吃啥");
        history.forget_current();
        assert_eq!(history.recent(10), "");
        history.set_application(Some("com.apple.Notes"));
        assert_eq!(history.recent(10), "会议纪要");
    }

    #[test]
    fn forgets_the_least_recent_application_beyond_the_limit() {
        let mut history = InputHistory::default();
        let start = Instant::now();
        for i in 0..=MAX_APPLICATIONS {
            history.set_application(Some(&format!("app{i}")));
            history.record_at("字", start + Duration::from_secs(i as u64));
        }
        assert_eq!(history.entries.len(), MAX_APPLICATIONS);
        history.set_application(Some("app0"));
        assert_eq!(history.text(), "");
        history.set_application(Some("app1"));
        assert_eq!(history.text(), "字");
    }
}
