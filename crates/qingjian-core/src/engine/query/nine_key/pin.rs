//! 拼音列里选定一个音节。

use crate::candidate::Candidate;
use crate::engine::Engine;
use crate::nine_key::{self, Shape};

impl Engine {
    /// 第一个没定下来的位置起的数字换成 `alternative` 的字母；后面还有数字就接一个 `'`，
    /// 免得定下来的 `xi` 和后面的 `2` 又拼成 `xia`。写法可以比这个位置短或长（拼音列里的别的切法）。
    /// 光标挪进中间时只看光标前的键，光标后的原样接上、光标停在选定的音节后。
    /// 写法对不上这里就不动，返回 `false`。
    pub fn pin_nine_key(&mut self, alternative: &str) -> bool {
        let text = self.composition.scope().to_owned();
        let rest = self.composition.rest().to_owned();
        let (shapes, _) = nine_key::segment(&text);
        let Some((_, position)) = shapes.first().and_then(|shape| shape.first_open()) else {
            return false;
        };
        let start = position.start;
        if !position.alternatives.iter().any(|a| a == alternative)
            && !nine_key::spellings_from(&text, start)
                .iter()
                .any(|a| a == alternative)
        {
            return false;
        }
        let end = start + alternative.len();
        let mut pinned = String::with_capacity(text.len() + 1);
        pinned.push_str(&text[..start]);
        pinned.push_str(alternative);
        if end < text.len() && !text[end..].starts_with('\'') {
            pinned.push('\'');
        }
        pinned.push_str(&text[end..]);
        let cursor = pinned.len();
        if !rest.is_empty() && !pinned.ends_with('\'') && !rest.starts_with('\'') {
            pinned.push('\'');
        }
        pinned.push_str(&rest);
        self.nine_key_unpinned = None;
        self.replace_nine_key_input(&pinned);
        if !rest.is_empty() {
            self.composition.move_to(cursor);
        }
        true
    }

    /// 退格碰到末尾选定的音节：先取消选定、整个退回成数字（`ni'hao` → `ni'426`），可以重选，之后再一个个删键；
    /// 记下用户选过的写法，退回后候选与拼音行仍按它排（[`Self::prefer_unpinned`]）。
    /// 光标挪进中间（[`Self::move_cursor_to_marked`]）时看光标前的那个音节，退回后光标停在这些数字后，
    /// 选定时自动接的 `'` 后面是数字就一起去掉（`ni|'426` → `64|426`）。
    /// 光标在开头或选定的音节中间、光标前是数字或不在九宫格时不动，返回 `false`。
    pub(in crate::engine) fn unpin_nine_key(&mut self) -> bool {
        let text = self.composition.text();
        let cursor = self.composition.cursor();
        if !self.nine_key || cursor == 0 {
            return false;
        }
        let (before, after) = text.split_at(cursor);
        if after.bytes().next().is_some_and(|b| b.is_ascii_lowercase()) {
            return false;
        }
        let start = before.rfind('\'').map_or(0, |i| i + 1);
        let last = &before[start..];
        if last.is_empty() || !last.bytes().all(|b| b.is_ascii_lowercase()) {
            return false;
        }
        let spelling = last.to_owned();
        let digits: String = last.chars().filter_map(nine_key::digit_of).collect();
        let after = match after.strip_prefix('\'') {
            Some(keys) if keys.bytes().next().is_some_and(|b| b.is_ascii_digit()) => keys,
            _ => after,
        };
        let cursor = start + digits.len();
        // 光标后记着的读法跟着挪：去掉的 `'` 让它们前移
        let shift = text.len() - before.len() - after.len();
        let mut kept = vec![(start, spelling)];
        if let Some((buffer, syllables)) = &self.nine_key_unpinned
            && buffer == text
        {
            kept.extend(
                syllables
                    .iter()
                    .filter(|(s, _)| *s >= before.len() + shift)
                    .map(|(s, spelling)| (s - shift, spelling.clone())),
            );
        }
        let input = format!("{}{digits}{after}", &before[..start]);
        let has_after = !after.is_empty();
        self.replace_nine_key_input(&input);
        if has_after {
            self.composition.move_to(cursor);
        }
        self.nine_key_unpinned = Some((input, kept));
        true
    }

    /// 退格删掉了 `before` 里光标前的一个键：记着的读法跟着删（删在音节里就去掉那个字母，后面的前移），
    /// 在中间删键时前后两段仍按原来的读法（`64|426` 删成 `6|426` 还是 n 与 hao）。
    pub(in crate::engine) fn follow_nine_key_deletion(&mut self, before: &str) {
        let text = self.composition.text();
        let at = self.composition.cursor();
        let removed = before.len() - text.len();
        let Some((buffer, syllables)) = &mut self.nine_key_unpinned else {
            return;
        };
        if buffer != before || removed == 0 {
            return;
        }
        let end = at + removed;
        let mut kept = Vec::with_capacity(syllables.len());
        for (start, spelling) in syllables.drain(..) {
            let stop = start + spelling.len();
            if stop <= at {
                kept.push((start, spelling));
            } else if start >= end {
                kept.push((start - removed, spelling));
            } else {
                let mut left = spelling[..at.saturating_sub(start)].to_owned();
                left.push_str(&spelling[end.min(stop) - start..]);
                if !left.is_empty() {
                    kept.push((start.min(at), left));
                }
            }
        }
        *syllables = kept;
        *buffer = text.to_owned();
    }

    /// 点拼音行把光标挪进九宫格的键中间（[`Self::move_cursor_to_marked`]）：各音节照挪之前拼音行上的读法记下，
    /// 候选只按光标前的键查时仍按它排、光标后那段仍按它显示（`ni'hao` 点在 ni 后，`64` 还读 ni、`426` 还读 hao，
    /// 不换成默认的 mi、gao）。`marked` 是挪之前拼音行上敲的那段（词库写法），一个字母对一个键。
    /// 键没变时（只挪过光标、或刚退回一个选定的音节）已经记着的音节不换，只补上没记的：
    /// 挪到半个音节里时拼音行已经换过读法，再记会越挪越偏。
    pub(in crate::engine) fn keep_nine_key_reading(&mut self, marked: &str) {
        let text = self.composition.text();
        let keys: Vec<usize> = text
            .char_indices()
            .filter(|(_, c)| *c != '\'')
            .map(|(byte, _)| byte)
            .collect();
        let mut syllables = Vec::new();
        let mut letters = 0;
        for syllable in marked.split('\'') {
            let Some(&start) = keys.get(letters) else {
                break;
            };
            syllables.push((start, syllable.to_owned()));
            letters += syllable.chars().count();
        }
        if let Some((buffer, kept)) = &mut self.nine_key_unpinned
            && buffer == text
        {
            let overlaps = |kept: &[(usize, String)], start: usize, len: usize| {
                kept.iter()
                    .any(|(s, spelling)| *s < start + len && start < s + spelling.len())
            };
            for (start, spelling) in syllables {
                if !overlaps(kept, start, spelling.len()) {
                    kept.push((start, spelling));
                }
            }
            return;
        }
        self.nine_key_unpinned = (!syllables.is_empty()).then(|| (text.to_owned(), syllables));
    }

    /// 要保住的读法（[`Self::unpin_nine_key`]、[`Self::keep_nine_key_reading`]）还在、之后只删过键或挪过光标：
    /// 读得上越多这些写法的候选越靠前（同样多的保持原来的相对顺序），拼音行跟着首选读。
    /// 删到某个位置只剩前几个键时按写法同样长的开头比（`64` 删成 `6` 还偏向 n）。
    pub(super) fn prefer_unpinned(&self, items: &mut [Candidate], shape: Option<&Shape>) {
        let Some((buffer, syllables)) = &self.nine_key_unpinned else {
            return;
        };
        let keys = self.composition.text();
        let Some(shape) = shape.filter(|_| buffer.starts_with(keys)) else {
            return;
        };
        let wanted: Vec<(usize, &str)> = shape
            .positions
            .iter()
            .enumerate()
            .filter_map(|(index, position)| {
                let spelling = kept_at(syllables, position.start)?;
                Some((index, &spelling[..position.len.min(spelling.len())]))
            })
            .collect();
        if wanted.is_empty() {
            return;
        }
        let reads = |c: &Candidate| {
            wanted
                .iter()
                .filter(|(index, prefix)| {
                    c.syllables
                        .get(*index)
                        .is_some_and(|s| s.starts_with(prefix))
                })
                .count()
        };
        items.sort_by_cached_key(|c| std::cmp::Reverse(reads(c)));
    }

    /// 选定音节后面的键删完了：选定时自动接的那个 `'` 一起去掉（`ni'` → `ni`），不留一个看不见的分隔要多按一下。
    pub(in crate::engine) fn drop_pin_separator(&mut self) {
        let text = self.composition.text();
        if self.composition.cursor() < text.len() {
            return;
        }
        let Some(before) = text.strip_suffix('\'') else {
            return;
        };
        let start = before.rfind('\'').map_or(0, |i| i + 1);
        if !self.nine_key
            || before.len() == start
            || !before[start..].bytes().all(|b| b.is_ascii_lowercase())
        {
            return;
        }
        let before = before.to_owned();
        self.replace_nine_key_input(&before);
    }

    /// 拼音行上从头起几个字是选定的音节（含它们之间的 `'`，不含前面留着的词）：缓冲区开头连着的全字母段。
    /// 选定只发生在第一个没定下来的位置，所以选定的总在前面。不在九宫格时为 0。
    pub fn nine_key_pinned(&self) -> usize {
        if !self.nine_key {
            return 0;
        }
        let mut pinned = 0;
        for (i, part) in self.composition.text().split('\'').enumerate() {
            if part.is_empty() || !part.bytes().all(|b| b.is_ascii_lowercase()) {
                break;
            }
            pinned += part.len() + usize::from(i > 0);
        }
        pinned
    }

    /// 换掉缓冲区、光标放到末尾，前面选好还没上屏的词留着（[`Self::set_input`] 会把它们清掉）。
    fn replace_nine_key_input(&mut self, input: &str) {
        let held = std::mem::take(&mut self.held);
        self.set_input(input);
        self.held = held;
    }
}

/// 记下的读法里缓冲区第 `at` 个字节起的那段：落在某个音节中间时取它后半（`hao` 从第二个键起是 `ao`）。
pub(super) fn kept_at(syllables: &[(usize, String)], at: usize) -> Option<&str> {
    syllables
        .iter()
        .find(|(start, spelling)| *start <= at && at < start + spelling.len())
        .map(|(start, spelling)| &spelling[at - start..])
}
