//! [`super::EmojiTable::lookup`] 的结果：一个词名下的条目，按表里的顺序。

use super::EmojiTable;

#[derive(Debug, Clone)]
pub struct Items<'a> {
    table: &'a EmojiTable,

    /// 下一条在表里的序号。
    next: u32,

    /// 最后一条之后的序号。
    end: u32,
}

impl<'a> Items<'a> {
    pub(super) fn new(table: &'a EmojiTable, start: u32, end: u32) -> Self {
        Self {
            table,
            next: start,
            end,
        }
    }
}

impl<'a> Iterator for Items<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        if self.next >= self.end {
            return None;
        }
        let item = self.table.item(self.next);
        self.next += 1;
        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = (self.end - self.next) as usize;
        (left, Some(left))
    }
}

impl ExactSizeIterator for Items<'_> {}
