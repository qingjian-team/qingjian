//! 拼音列里选定一个音节。

use crate::candidate::Candidate;
use crate::engine::Engine;
use crate::nine_key::{self, Shape};

impl Engine {
    /// 第一个没定下来的位置起的数字换成 `alternative` 的字母；后面还有数字就接一个 `'`，
    /// 免得定下来的 `xi` 和后面的 `2` 又拼成 `xia`。写法可以比这个位置短或长（拼音列里的别的切法）。
    /// 写法对不上这里就不动，返回 `false`。
    pub fn pin_nine_key(&mut self, alternative: &str) -> bool {
        let text = self.composition.text().to_owned();
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
        self.nine_key_unpinned = None;
        self.replace_nine_key_input(&pinned);
        true
    }

    /// 退格碰到末尾选定的音节：先取消选定、整个退回成数字（`ni'hao` → `ni'426`），可以重选，之后再一个个删键；
    /// 记下用户选过的写法，退回后候选与拼音行仍按它排（[`Self::prefer_unpinned`]）。
    /// 光标不在末尾、末尾是数字或不在九宫格时不动，返回 `false`。
    pub(in crate::engine) fn unpin_nine_key(&mut self) -> bool {
        let text = self.composition.text();
        if !self.nine_key || self.composition.cursor() < text.len() {
            return false;
        }
        let start = text.rfind('\'').map_or(0, |i| i + 1);
        let last = &text[start..];
        if last.is_empty() || !last.bytes().all(|b| b.is_ascii_lowercase()) {
            return false;
        }
        let spelling = last.to_owned();
        let digits: String = last.chars().filter_map(nine_key::digit_of).collect();
        let rest = format!("{}{digits}", &text[..start]);
        self.replace_nine_key_input(&rest);
        self.nine_key_unpinned = Some((rest, start, spelling));
        true
    }

    /// 刚退回成数字的音节（[`Self::unpin_nine_key`]）还在、之后只删过键：把读成用户选过写法的候选挪到前面（保持原来的相对顺序），
    /// 拼音行跟着首选读。删到这个位置只剩前几个键时按写法同样长的开头比（`64` 删成 `6` 还偏向 n）。
    pub(super) fn prefer_unpinned(&self, items: &mut Vec<Candidate>, shape: Option<&Shape>) {
        let Some((buffer, start, spelling)) = &self.nine_key_unpinned else {
            return;
        };
        let keys = self.composition.text();
        if !buffer.starts_with(keys) || keys.len() <= *start {
            return;
        }
        let Some((index, position)) = shape.and_then(|shape| {
            shape
                .positions
                .iter()
                .enumerate()
                .find(|(_, position)| position.start == *start)
        }) else {
            return;
        };
        let prefix = &spelling[..position.len.min(spelling.len())];
        let reads = |c: &Candidate| {
            c.syllables
                .get(index)
                .is_some_and(|s| s.starts_with(prefix))
        };
        let (mut preferred, others): (Vec<Candidate>, Vec<Candidate>) =
            std::mem::take(items).into_iter().partition(reads);
        preferred.extend(others);
        *items = preferred;
    }

    /// 选定音节后面的键删完了：选定时自动接的那个 `'` 一起去掉（`ni'` → `ni`），不留一个看不见的分隔要多按一下。
    pub(in crate::engine) fn drop_pin_separator(&mut self) {
        let text = self.composition.text();
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
