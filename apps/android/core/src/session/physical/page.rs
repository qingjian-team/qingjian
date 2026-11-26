//! 物理键盘候选页：数字总是指当前页，内容更新后回到第一页。

use crate::frame::{Frame, FrameCandidate};

#[derive(Default)]
pub struct PhysicalPage {
    pub(super) items: Vec<FrameCandidate>,

    preedit: String,

    cursor: usize,

    expression: bool,

    single_chars: bool,

    pinyin_choices: Vec<String>,

    pinned: (usize, usize),

    start: usize,

    highlighted: usize,

    pub(super) deleting: bool,

    pub(super) navigated: bool,
}

impl PhysicalPage {
    pub const SIZE: usize = 5;

    pub fn show(&mut self, frame: &Frame) {
        // 内容相同的轮询结果保留键盘导航位置。
        if self.preedit != frame.preedit
            || self
                .items
                .iter()
                .map(|c| &c.text)
                .ne(frame.candidates.iter().map(|c| &c.text))
        {
            self.start = 0;
            self.highlighted = 0;
            self.navigated = false;
        }
        self.items = frame.candidates.clone();
        self.preedit.clone_from(&frame.preedit);
        self.cursor = frame.preedit_cursor;
        self.pinned = frame.preedit_pinned;
        self.expression = frame.expression;
        self.single_chars = frame.single_chars;
        self.pinyin_choices.clone_from(&frame.pinyin_choices);
    }

    pub fn selected(&self) -> usize {
        self.start + self.highlighted
    }

    pub fn index(&self, digit: usize) -> Option<usize> {
        (digit < Self::SIZE && self.start + digit < self.items.len()).then_some(self.start + digit)
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn highlighted(&self) -> usize {
        self.highlighted
    }

    pub fn previous(&self) -> bool {
        self.start > 0
    }

    pub fn next(&self) -> bool {
        self.start + Self::SIZE < self.items.len()
    }

    pub fn turn(&mut self, step: isize) {
        self.navigated = true;
        if step < 0 && self.previous() {
            self.start -= Self::SIZE;
        }
        if step > 0 && self.next() {
            self.start += Self::SIZE;
        }
        self.highlighted = 0;
    }

    pub fn step(&mut self, step: isize) {
        self.navigated = true;
        let index = self
            .selected()
            .saturating_add_signed(step)
            .min(self.items.len().saturating_sub(1));
        self.start = index / Self::SIZE * Self::SIZE;
        self.highlighted = index % Self::SIZE;
    }

    pub fn frame(&self) -> Frame {
        Frame {
            commit: String::new(),
            replace: 0,
            preedit: self.preedit.clone(),
            preedit_cursor: self.cursor,
            preedit_pinned: self.pinned,
            notice: String::new(),
            pinyin_choices: self.pinyin_choices.clone(),
            candidates: self.items.clone(),
            expression: self.expression,
            single_chars: self.single_chars,
        }
    }
}
