//! 物理输入状态、按键编辑与选词调用。

use crate::frame::Frame;
use crate::session::Session;

impl Session {
    /// 触屏与物理输入共用引擎，但上屏后的候选条仅属于触屏。
    pub fn set_physical_input(&mut self, active: bool) -> Frame {
        self.physical_input = active;
        if active && self.preedit.is_empty() {
            self.refresh(String::new())
        } else {
            self.frame(String::new())
        }
    }

    /// `action` 对应 Android 壳的 Action；没有处理时返回 None，让应用收到原事件。
    pub fn physical_key(
        &mut self,
        action: i32,
        character: Option<char>,
        repeat: bool,
    ) -> Option<Frame> {
        if !self.physical_input {
            self.set_physical_input(true);
        }
        let composing = !self.preedit.is_empty();
        let has_candidates = composing || !self.completions.is_empty();
        if action == 3 && !repeat {
            self.physical.deleting = composing;
        }
        match action {
            0 => return self.physical_character(character?),
            1 if has_candidates => {
                if self.engine.zhuyin_needs_tone() {
                    return Some(self.push(' '));
                }
                if self.physical.items.is_empty() {
                    return composing.then(|| self.commit_raw());
                }
                return Some(self.select(self.physical.selected()));
            }
            2 if composing => return Some(self.commit_raw()),
            3 if composing || repeat && self.physical.deleting => {
                self.physical.deleting = true;
                if let Some(kana) = &mut self.kana {
                    kana.backspace();
                } else {
                    self.engine.backspace(repeat);
                }
            }
            4 if composing && self.kana.is_none() => {
                self.engine.delete_forward();
            }
            5 if composing || !self.completions.is_empty() => return Some(self.clear()),
            6 if composing && self.kana.is_none() => {
                self.engine.move_cursor_left();
            }
            7 if composing && self.kana.is_none() => {
                self.engine.move_cursor_right();
            }
            8 if composing && self.kana.is_none() => self.engine.move_cursor_home(),
            9 if composing && self.kana.is_none() => self.engine.move_cursor_end(),
            10 | 11 if has_candidates && !self.physical.items.is_empty() => {
                self.physical.turn(if action == 10 { -1 } else { 1 });
                self.engine.note_page_turn();
                return Some(self.physical.frame());
            }
            12 | 13 if has_candidates && !self.physical.items.is_empty() => {
                self.physical.step(if action == 12 { -1 } else { 1 });
                return Some(self.physical.frame());
            }
            _ => return None,
        }
        Some(self.refresh(String::new()))
    }

    fn physical_character(&mut self, c: char) -> Option<Frame> {
        let composing = !self.preedit.is_empty();
        self.physical.deleting = false;
        if composing
            && self.engine.is_zhuyin_mode()
            && self.physical.navigated
            && let Some(digit) = c.to_digit(10).filter(|_| c.is_ascii_digit())
        {
            if digit > 0
                && let Some(index) = self.physical.index(digit as usize - 1)
            {
                return Some(self.select(index));
            }
            return Some(self.frame(String::new()));
        }
        // 数字和标点在算式、码点、注音与双拼中的含义由 Core 定。
        let scheme_key = self.kana.is_none()
            && (self.engine.expression_input(c)
                || self.engine.unicode_entry() && c.is_ascii_hexdigit()
                || self.engine.is_zhuyin_mode() && qingjian_core::zhuyin::map_key(c).is_some()
                || self
                    .engine
                    .shuangpin()
                    .is_some_and(|layout| layout.is_key(c)));
        if scheme_key
            || c.is_ascii_alphabetic()
            || composing && (c == '\'' || self.engine.raw_mode())
        {
            if c.is_ascii_uppercase()
                && !self.engine.english_mode()
                && !self.engine.takes_mode_letter(c)
            {
                if !composing {
                    self.engine.note_passthrough(c);
                }
                return None;
            }
            return Some(self.push(c));
        }
        if let Some(digit) = c.to_digit(10).filter(|_| c.is_ascii_digit()) {
            if (composing || !self.completions.is_empty())
                && digit > 0
                && let Some(index) = self.physical.index(digit as usize - 1)
            {
                return Some(self.select(index));
            }
            if composing && self.engine.english_mode() {
                return Some(self.push(c));
            }
            if composing {
                return Some(self.frame(String::new()));
            }
        }
        if !composing && c.is_ascii_digit() {
            self.engine.note_passthrough(c);
        }
        None
    }

    /// 每次内容更新后同步物理候选，键盘导航只修改页与高亮，不重查词库。
    pub(crate) fn note_physical(&mut self, frame: &Frame) {
        self.physical.show(frame);
    }
}
