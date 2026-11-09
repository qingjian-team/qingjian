//! 成对符号（#928）：敲左半边时连右半边一起补上、光标停在中间；紧接着敲右半边就跳过补上的那个，
//! 刚补完、中间还什么都没打就退格，两个一起删。
//!
//! 这里只有规则和「补过、还没跳过的右半边」的记录。光标后面是什么只有壳读得到：跳过与删一对由壳核对文档里
//! 真是这个字再做，对不上就照常插入 / 照常退格，所以用户自己挪了光标也不会删错字。

/// 全角的左右半边：中文与日文标点。
const FULL_WIDTH: [(char, char); 9] = [
    ('（', '）'),
    ('【', '】'),
    ('「', '」'),
    ('『', '』'),
    ('《', '》'),
    ('〈', '〉'),
    ('〔', '〕'),
    ('“', '”'),
    ('‘', '’'),
];

/// 半角的左右半边（英文）。不含 `'`（don't 里的撇号）与 `<`（比较号），补了反而要删。
const HALF_WIDTH: [(char, char); 4] = [('(', ')'), ('[', ']'), ('{', '}'), ('"', '"')];

/// 最多记几层补过的右半边；再多就丢最早的（嵌套到这么深的多半早就挪过光标了）。
const MAX_OPEN: usize = 16;

/// 要壳对文档做的事。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairEdit {
    /// 上屏 `text`（左右两半），光标退回 `back` 个字符停在中间。
    Insert { text: String, back: usize },

    /// 光标后面正是 `closer`（补上的那个）就跳过它，不是就照常上屏它。
    Skip { closer: char },
}

/// 补过、还没跳过的右半边。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Pairs {
    /// 由外到内，最后一个是光标后面最近的那个。
    open: Vec<char>,

    /// 刚补完一对、中间还什么都没打：这时退格两个一起删。
    fresh: bool,
}

impl Pairs {
    /// 一个符号 `text` 要上屏了（已按当前模式转好全半角）。是补过的右半边就跳过；
    /// `enabled`（当前模式开着成对）且是左半边就连右半边一起上屏；别的为 `None`，照常上屏。
    pub fn edit(&mut self, text: &str, enabled: bool) -> Option<PairEdit> {
        self.fresh = false;
        let mut chars = text.chars();
        let (Some(c), None) = (chars.next(), chars.next()) else {
            return None;
        };
        // 先认右半边：英文引号左右同形，补过的那个就是要跳的
        if self.open.last() == Some(&c) {
            self.open.pop();
            return Some(PairEdit::Skip { closer: c });
        }
        if !enabled {
            return None;
        }
        let closer = closer_of(c)?;
        if self.open.len() == MAX_OPEN {
            self.open.remove(0);
        }
        self.open.push(closer);
        self.fresh = true;
        Some(PairEdit::Insert {
            text: format!("{c}{closer}"),
            back: 1,
        })
    }

    /// 光标后最近的那个补过的右半边正是 `closer`。
    pub fn closes(&self, closer: char) -> bool {
        self.open.last() == Some(&closer)
    }

    /// 没在组句时按了退格：刚补完一对、中间什么都没打，返回这一对（左, 右），壳核对光标前后正是它们再一起删。
    pub fn backspace(&mut self) -> Option<(char, char)> {
        if !std::mem::take(&mut self.fresh) {
            return None;
        }
        let closer = self.open.pop()?;
        Some((opener_of(closer)?, closer))
    }

    /// 补完之后又上屏了别的（字、词、没成对的标点）：中间有内容了，退格不再删一对。
    pub fn note_typed(&mut self) {
        self.fresh = false;
    }

    /// 光标离开了（切应用、点别处）：补过的右半边都不再认。
    pub fn clear(&mut self) {
        self.open.clear();
        self.fresh = false;
    }
}

fn closer_of(opener: char) -> Option<char> {
    FULL_WIDTH
        .iter()
        .chain(&HALF_WIDTH)
        .find(|(open, _)| *open == opener)
        .map(|(_, close)| *close)
}

fn opener_of(closer: char) -> Option<char> {
    FULL_WIDTH
        .iter()
        .chain(&HALF_WIDTH)
        .find(|(_, close)| *close == closer)
        .map(|(open, _)| *open)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert(text: &str) -> Option<PairEdit> {
        Some(PairEdit::Insert {
            text: text.to_owned(),
            back: 1,
        })
    }

    #[test]
    fn openers_bring_their_closer_and_the_closer_is_skipped() {
        let mut pairs = Pairs::default();
        assert_eq!(pairs.edit("（", true), insert("（）"));
        assert_eq!(pairs.edit("《", true), insert("《》"));
        pairs.note_typed();
        assert_eq!(
            pairs.edit("》", true),
            Some(PairEdit::Skip { closer: '》' })
        );
        assert_eq!(
            pairs.edit("）", true),
            Some(PairEdit::Skip { closer: '）' })
        );
        // 没补过的右半边照常上屏
        assert_eq!(pairs.edit("）", true), None);
        assert_eq!(pairs.edit("，", true), None);
    }

    #[test]
    fn half_width_pairs_skip_quotes_that_look_the_same() {
        let mut pairs = Pairs::default();
        assert_eq!(pairs.edit("\"", true), insert("\"\""));
        assert_eq!(pairs.edit("\"", true), Some(PairEdit::Skip { closer: '"' }));
        assert_eq!(pairs.edit("(", true), insert("()"));
        // 撇号与比较号不成对
        assert_eq!(pairs.edit("'", true), None);
        assert_eq!(pairs.edit("<", true), None);
    }

    #[test]
    fn disabled_mode_still_skips_closers_it_inserted() {
        let mut pairs = Pairs::default();
        assert_eq!(pairs.edit("「", false), None);
        assert_eq!(pairs.edit("「", true), insert("「」"));
        assert_eq!(
            pairs.edit("」", false),
            Some(PairEdit::Skip { closer: '」' })
        );
    }

    #[test]
    fn backspace_deletes_the_pair_only_right_after_inserting_it() {
        let mut pairs = Pairs::default();
        pairs.edit("【", true);
        assert_eq!(pairs.backspace(), Some(('【', '】')));
        assert_eq!(pairs.backspace(), None);
        pairs.edit("“", true);
        pairs.note_typed();
        assert_eq!(pairs.backspace(), None);
        // 中间打过字，右半边仍然认得
        assert_eq!(pairs.edit("”", true), Some(PairEdit::Skip { closer: '”' }));
    }

    #[test]
    fn clearing_forgets_open_pairs() {
        let mut pairs = Pairs::default();
        pairs.edit("（", true);
        pairs.clear();
        assert_eq!(pairs.edit("）", true), None);
        assert_eq!(pairs.backspace(), None);
    }
}
