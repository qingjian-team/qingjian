//! 点拼音行挪光标（#881）：拼音行上第几个字之后 → 缓冲区里的光标位置。
//!
//! 拼音行与敲的键不总是一一对应：全拼多出切分的 `'`，双拼显示解出来的全拼。全拼按字母数对，
//! 双拼按音节对（落到最近的音节边界）；纠错、辅码、注音、九宫格对不上，不挪。

use super::composing::unit_len_after;
use super::{Engine, Query};

impl Engine {
    /// 光标挪到拼音行（[`Query::marked_text`]）第 `index` 个字之后对应的位置；`query` 是这一帧的查询。
    /// 对不上的写法（见文件头）返回 `false`，光标不动。
    pub fn move_cursor_to_marked(&mut self, query: &Query, index: usize) -> bool {
        if query.text != self.composition.text() {
            return false;
        }
        let Some(cursor) = text_cursor(query, index, self.shuangpin.is_some()) else {
            return false;
        };
        self.composition.move_to(cursor);
        true
    }
}

fn text_cursor(query: &Query, index: usize, shuangpin: bool) -> Option<usize> {
    if query.correction.is_some() || query.aux.is_some() {
        return None;
    }
    let marked: Vec<char> = query.marked_text().chars().collect();
    let held = query.held.chars().count();
    let index = index.min(marked.len());
    if index <= held {
        return Some(0);
    }
    let typed = &marked[held..];
    let at = index - held;
    let text = query.text.as_str();
    if shuangpin && query.shuangpin_raw_preedit {
        return Some(
            text.char_indices()
                .nth(at)
                .map_or(text.len(), |(byte, _)| byte),
        );
    }
    match (query.decoded_keys, shuangpin) {
        (false, _) => Some(letter_cursor(text, typed, at)),
        (true, true) => Some(syllable_cursor(text, typed, at)),
        (true, false) => None,
    }
}

/// 拼音行与缓冲区只差 `'`：数拼音行上点的位置之前有几个字母，缓冲区里走过同样多的字母。
fn letter_cursor(text: &str, typed: &[char], at: usize) -> usize {
    let letters = typed[..at].iter().filter(|c| **c != '\'').count();
    let mut seen = 0;
    let mut cursor = text.len();
    for (byte, c) in text.char_indices() {
        if seen == letters && (c != '\'' || typed[at - 1] != '\'') {
            cursor = byte;
            break;
        }
        if c != '\'' {
            seen += 1;
        }
    }
    cursor
}

/// 双拼两键一音节：点在音节前半落到它前面，后半落到它后面，再按音节在缓冲区里走过去。
fn syllable_cursor(text: &str, typed: &[char], at: usize) -> usize {
    let mut syllables = 0;
    let mut start = 0;
    for (i, c) in typed.iter().chain(['\''].iter()).enumerate() {
        if *c != '\'' {
            continue;
        }
        if at >= i {
            syllables += 1;
        } else if at > start && (at - start) * 2 > i - start {
            syllables += 1;
            break;
        } else {
            break;
        }
        start = i + 1;
    }
    let mut cursor = 0;
    for _ in 0..syllables {
        let step = unit_len_after(&text[cursor..], true, false);
        if step == 0 {
            break;
        }
        cursor += step;
    }
    cursor
}
