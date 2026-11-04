//! 翻译框里的原文和它的翻译进度：原文带一个光标和选区，像记事本那样在光标处插字、删字，有选区时先删掉选中的字；
//! 停键满 [`DEBOUNCE`] 才发，原文或目标语言没变不重发，没翻出来的过 [`RETRY_AFTER`] 自动重试一次；
//! 新译文回来前先留着上一条，免得应用里的 preedit 一闪一闪。

use std::ops::Range;
use std::time::{Duration, Instant};

use super::{TranslateJob, TranslateStatus};
use crate::Language;
use crate::engine::prediction::translation_target;

/// 停键这么久才发请求：打字过程中不一键一发。
pub(crate) const DEBOUNCE: Duration = Duration::from_millis(800);

/// 没翻出来（多半是网络抖了一下）过这么久自动再发一次；再失败就等用户按回车。
pub(crate) const RETRY_AFTER: Duration = Duration::from_secs(2);

/// 原文少于这么多字不发：一个字翻出来没多大用，还白费一次请求。
const MIN_CHARS: usize = 2;

/// 一次没翻出来的内容。
#[derive(Debug, Clone)]
struct Failure {
    job: TranslateJob,

    at: Instant,

    /// 自动重试过了：再失败就不再自己发。
    retried: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TranslateDraft {
    source: String,

    /// 光标在原文里的位置（第几个字之后）。
    cursor: usize,

    /// 选区的另一头（拖选或按住 Shift 时光标出发的位置）；与光标重合或为 `None` 时没有选区。
    anchor: Option<usize>,

    /// 最后一次改原文的时刻；离现在不满 [`DEBOUNCE`] 就先不发。
    edited: Option<Instant>,

    /// 在飞的请求序号与内容。
    request: Option<(u64, TranslateJob)>,

    /// 最近一次拿到的译文与它对应的内容。
    result: Option<(TranslateJob, String)>,

    /// 最近一次没翻出来的内容。
    failed: Option<Failure>,

    /// 译文或失败有了新消息、壳还没重画。
    unseen: bool,
}

impl TranslateDraft {
    pub(crate) fn source(&self) -> &str {
        &self.source
    }

    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    /// 光标处的字节位置。
    fn byte_at(&self, chars: usize) -> usize {
        self.source
            .char_indices()
            .nth(chars)
            .map_or(self.source.len(), |(i, _)| i)
    }

    fn char_count(&self) -> usize {
        self.source.chars().count()
    }

    /// 选中的字（按字算，从小到大）；没选中东西时为 `None`。
    pub(crate) fn selection(&self) -> Option<Range<usize>> {
        let anchor = self.anchor?;
        (anchor != self.cursor).then(|| anchor.min(self.cursor)..anchor.max(self.cursor))
    }

    /// 删掉选中的字，光标停在原来选区开头。没选中东西时返回 `false`。
    pub(crate) fn remove_selection(&mut self, now: Instant) -> bool {
        let selection = self.selection();
        self.anchor = None;
        let Some(range) = selection else {
            return false;
        };
        let (start, end) = (self.byte_at(range.start), self.byte_at(range.end));
        self.source.replace_range(start..end, "");
        self.cursor = range.start;
        self.edited = Some(now);
        true
    }

    /// 在光标处插字（有选区就替换掉选中的字），光标挪到插入的字后面。
    pub(crate) fn push(&mut self, text: &str, now: Instant) {
        if text.is_empty() {
            return;
        }
        self.remove_selection(now);
        let at = self.byte_at(self.cursor);
        self.source.insert_str(at, text);
        self.cursor += text.chars().count();
        self.edited = Some(now);
    }

    /// 删掉选中的字，没选中就删光标前一个字。什么都没删（光标在最前面）时返回 `false`。
    pub(crate) fn pop(&mut self, now: Instant) -> bool {
        if self.remove_selection(now) {
            return true;
        }
        if self.cursor == 0 {
            return false;
        }
        let at = self.byte_at(self.cursor - 1);
        self.source.remove(at);
        self.cursor -= 1;
        self.edited = Some(now);
        true
    }

    /// Delete：删掉选中的字，没选中就删光标后一个字。什么都没删（光标在最后）时返回 `false`。
    pub(crate) fn delete(&mut self, now: Instant) -> bool {
        if self.remove_selection(now) {
            return true;
        }
        if self.cursor >= self.char_count() {
            return false;
        }
        let at = self.byte_at(self.cursor);
        self.source.remove(at);
        self.edited = Some(now);
        true
    }

    /// 光标挪到第 `position` 个字之后（超出就到最后）。`extend` 时选区从原来的光标拉到这里（拖选、Shift），
    /// 否则取消选区。返回光标或选区变没变。
    pub(crate) fn place_cursor(&mut self, position: usize, extend: bool) -> bool {
        let position = position.min(self.char_count());
        let before = (self.cursor, self.selection());
        if extend {
            self.anchor.get_or_insert(self.cursor);
        } else {
            self.anchor = None;
        }
        self.cursor = position;
        before != (self.cursor, self.selection())
    }

    /// 光标左右挪 `delta` 个字。不扩选区时有选区就只是收起选区，光标停在挪的那一头（同记事本）。返回变没变。
    pub(crate) fn move_cursor(&mut self, delta: isize, extend: bool) -> bool {
        if !extend && let Some(range) = self.selection() {
            let edge = if delta < 0 { range.start } else { range.end };
            return self.place_cursor(edge, false);
        }
        let position = (self.cursor as isize + delta).max(0) as usize;
        self.place_cursor(position, extend)
    }

    /// 全选。返回变没变。
    pub(crate) fn select_all(&mut self) -> bool {
        let before = (self.cursor, self.selection());
        self.anchor = Some(0);
        self.cursor = self.char_count();
        before != (self.cursor, self.selection())
    }

    /// 不等停键，下一次 [`Self::due`] 就发；没翻出来的也重新发（回车时译文还没到）。
    pub(crate) fn hurry(&mut self) {
        self.edited = None;
        self.failed = None;
    }

    /// 框里的原文按 `chosen` 该怎么译；字太少返回 `None`。
    pub(crate) fn job(&self, chosen: Language) -> Option<TranslateJob> {
        let source = self.source.trim();
        if source.chars().count() < MIN_CHARS {
            return None;
        }
        Some(TranslateJob {
            source: source.to_owned(),
            language: translation_target(source, chosen),
        })
    }

    /// 这份内容没翻出来、而且不再自动重试。
    fn gave_up(&self, job: &TranslateJob, now: Instant) -> bool {
        self.failed.as_ref().is_some_and(|failure| {
            failure.job == *job && (failure.retried || now.duration_since(failure.at) < RETRY_AFTER)
        })
    }

    /// 到时候该发的请求：停键够久、而且这份内容没译过、没在飞，没翻出来的只自动重试一次。
    pub(crate) fn due(&self, now: Instant, chosen: Language) -> Option<TranslateJob> {
        if self
            .edited
            .is_some_and(|edited| now.duration_since(edited) < DEBOUNCE)
        {
            return None;
        }
        let job = self.job(chosen)?;
        let settled = self.result.as_ref().map(|(j, _)| j) == Some(&job)
            || self.request.as_ref().map(|(_, j)| j) == Some(&job)
            || self.gave_up(&job, now);
        (!settled).then_some(job)
    }

    pub(crate) fn sent(&mut self, sequence: u64, job: TranslateJob) {
        if let Some(failure) = self.failed.as_mut().filter(|f| f.job == job) {
            failure.retried = true;
        }
        self.request = Some((sequence, job));
    }

    pub(crate) fn resolve(&mut self, job: TranslateJob, text: String) {
        if self.failed.as_ref().is_some_and(|f| f.job == job) {
            self.failed = None;
        }
        self.result = Some((job, text));
        self.unseen = true;
    }

    pub(crate) fn fail(&mut self, job: TranslateJob, now: Instant) {
        let retried = self
            .failed
            .as_ref()
            .is_some_and(|f| f.job == job && f.retried);
        self.failed = Some(Failure {
            job,
            at: now,
            retried,
        });
        self.unseen = true;
    }

    /// 没翻出来、还会自动再试一次：壳要接着轮询。
    pub(crate) fn retry_pending(&self) -> bool {
        self.failed.as_ref().is_some_and(|failure| !failure.retried)
    }

    /// 上次问过之后有没有新消息要重画。
    pub(crate) fn take_unseen(&mut self) -> bool {
        std::mem::take(&mut self.unseen)
    }

    /// 在飞的请求序号。
    pub(crate) fn in_flight(&self) -> Option<u64> {
        self.request.as_ref().map(|(sequence, _)| *sequence)
    }

    /// 在飞的请求被作废了（上层清掉了全部联想）：忘掉它，下一次 [`Self::due`] 重发。
    pub(crate) fn forget_request(&mut self) {
        self.request = None;
    }

    /// 收到序号 `sequence` 的结果：是这个框发的就记下并返回对应的内容，不是返回 `None`。
    pub(crate) fn answer(
        &mut self,
        sequence: u64,
        text: Option<String>,
        now: Instant,
    ) -> Option<(TranslateJob, Option<String>)> {
        if self.in_flight() != Some(sequence) {
            return None;
        }
        let (_, job) = self.request.take()?;
        match text.filter(|t| !t.trim().is_empty()) {
            Some(text) => {
                self.resolve(job.clone(), text.clone());
                Some((job, Some(text)))
            }
            None => {
                self.fail(job.clone(), now);
                Some((job, None))
            }
        }
    }

    /// 要放进应用 preedit 的译文：最近一条，原文改了还在等新的时先留着；原文删光或这次没翻出来就不显示，免得对不上。
    pub(crate) fn translation(&self, chosen: Language) -> Option<&str> {
        if self.source.trim().is_empty() || self.status(chosen) == TranslateStatus::Failed {
            return None;
        }
        self.result.as_ref().map(|(_, text)| text.as_str())
    }

    /// 对得上框里原文与 `chosen` 的译文。
    pub(crate) fn ready(&self, chosen: Language) -> Option<&str> {
        let job = self.job(chosen)?;
        self.result
            .as_ref()
            .filter(|(j, _)| *j == job)
            .map(|(_, text)| text.as_str())
    }

    pub(crate) fn status(&self, chosen: Language) -> TranslateStatus {
        let Some(job) = self.job(chosen) else {
            return TranslateStatus::Idle;
        };
        if self.ready(chosen).is_some() {
            TranslateStatus::Ready
        } else if self.request.as_ref().is_some_and(|(_, j)| *j == job) {
            TranslateStatus::Waiting
        } else if self.failed.as_ref().is_some_and(|f| f.job == job) {
            TranslateStatus::Failed
        } else {
            TranslateStatus::Waiting
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_for_a_pause_and_does_not_resend_the_same_text() {
        let start = Instant::now();
        let mut draft = TranslateDraft::default();
        draft.push("你", start);
        assert_eq!(draft.due(start + DEBOUNCE, Language::English), None);
        draft.push("好", start);
        assert_eq!(draft.status(Language::English), TranslateStatus::Waiting);
        assert_eq!(draft.due(start, Language::English), None);
        let job = draft.due(start + DEBOUNCE, Language::English).unwrap();
        assert_eq!(job.source, "你好");
        draft.sent(7, job.clone());
        assert_eq!(draft.due(start + DEBOUNCE, Language::English), None);
        // 别人的结果不算
        assert_eq!(draft.answer(6, Some("x".to_owned()), start), None);
        draft.answer(7, Some("Hello".to_owned()), start);
        assert_eq!(draft.ready(Language::English), Some("Hello"));
        assert_eq!(draft.due(start + DEBOUNCE, Language::English), None);
        // 换了语言要重发，旧译文先留着
        assert!(draft.due(start + DEBOUNCE, Language::Japanese).is_some());
        assert_eq!(draft.translation(Language::Japanese), Some("Hello"));
        assert_eq!(draft.status(Language::Japanese), TranslateStatus::Waiting);
    }

    #[test]
    fn foreign_text_goes_back_to_chinese_and_a_failure_is_retried_once() {
        let start = Instant::now();
        let mut draft = TranslateDraft::default();
        draft.push("good morning", start);
        let job = draft.due(start + DEBOUNCE, Language::English).unwrap();
        assert_eq!(job.language, Language::Chinese);
        draft.sent(1, job.clone());
        draft.answer(1, None, start);
        assert_eq!(draft.status(Language::English), TranslateStatus::Failed);
        assert_eq!(draft.due(start + DEBOUNCE, Language::English), None);
        // 过一会儿自己重试一次，再失败就不再发
        let later = start + RETRY_AFTER;
        assert_eq!(draft.due(later, Language::English), Some(job.clone()));
        draft.sent(2, job.clone());
        assert_eq!(draft.status(Language::English), TranslateStatus::Waiting);
        draft.answer(2, None, later);
        assert_eq!(draft.due(later + RETRY_AFTER, Language::English), None);
        // 回车重新发
        draft.hurry();
        assert_eq!(draft.due(later, Language::English), Some(job));
    }

    #[test]
    fn edits_happen_at_the_cursor() {
        let now = Instant::now();
        let mut draft = TranslateDraft::default();
        draft.push("你世界", now);
        assert!(draft.place_cursor(1, false));
        draft.push("好", now);
        assert_eq!((draft.source(), draft.cursor()), ("你好世界", 2));
        assert!(draft.delete(now));
        assert_eq!(draft.source(), "你好界");
        assert!(draft.pop(now));
        assert_eq!((draft.source(), draft.cursor()), ("你界", 1));
        assert!(draft.move_cursor(-5, false));
        assert!(!draft.pop(now));
        assert!(!draft.place_cursor(0, false));
        assert!(draft.place_cursor(99, false));
        assert_eq!(draft.cursor(), 2);
        assert!(!draft.delete(now));
    }

    #[test]
    fn a_selection_is_replaced_or_removed_as_a_whole() {
        let now = Instant::now();
        let mut draft = TranslateDraft::default();
        draft.push("今天天气很好", now);
        // 从最后往左拖到第 4 个字
        assert!(draft.place_cursor(4, true));
        assert_eq!(draft.selection(), Some(4..6));
        draft.push("不错", now);
        assert_eq!((draft.source(), draft.cursor()), ("今天天气不错", 6));
        assert_eq!(draft.selection(), None);
        // Shift+→ 两下选两个字，退格整段删掉
        draft.place_cursor(2, false);
        draft.move_cursor(1, true);
        draft.move_cursor(1, true);
        assert_eq!(draft.selection(), Some(2..4));
        assert!(draft.pop(now));
        assert_eq!((draft.source(), draft.cursor()), ("今天不错", 2));
        // 不按 Shift 的 ←→ 只收起选区
        draft.place_cursor(0, false);
        draft.place_cursor(3, true);
        assert!(draft.move_cursor(-1, false));
        assert_eq!((draft.cursor(), draft.selection()), (0, None));
        assert!(draft.select_all());
        assert!(!draft.select_all());
        assert!(draft.delete(now));
        assert_eq!(draft.source(), "");
    }
}
