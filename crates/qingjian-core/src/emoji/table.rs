//! 词 → emoji 列表。
//!
//! 条目全拼进一块文字缓冲，`bounds` 记每条的起点，词只记自己那几条的序号区间（每个 emoji 一个 `String` 的话，
//! 小分配的开销是内容本身的十几倍）。合并、筛选只在启动时做，拆回列表做完再拼回去。

use std::collections::HashMap;
use std::path::Path;

use super::items::Items;

/// 表里一个词后面的条目按相关度排好（名字正好是这个词的在前）。
#[derive(Debug, Default)]
pub struct EmojiTable {
    /// 词 → 它的条目在 `bounds` 里的序号区间 `[起, 止)`。
    words: HashMap<Box<str>, (u32, u32)>,

    /// 所有条目拼在一起。
    text: String,

    /// 第 i 条是 `text[bounds[i]..bounds[i + 1]]`；非空表以 0 开头。
    bounds: Vec<u32>,
}

/// 拆开的样子：词 → 条目，供解析、合并、筛选时改。
type Lists = HashMap<String, Vec<String>>;

impl EmojiTable {
    /// 解析 TSV：`词\temoji emoji …`，空行与 `#` 开头跳过；格式不对的行返回行号。
    pub fn parse(source: &str) -> Result<Self, usize> {
        let mut lists = Lists::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (word, emojis) = line.split_once('\t').ok_or(index + 1)?;
            let list = lists.entry(word.to_owned()).or_default();
            for emoji in emojis.split(' ').filter(|e| !e.is_empty()) {
                if !list.iter().any(|e| e == emoji) {
                    list.push(emoji.to_owned());
                }
            }
        }
        Ok(Self::pack(lists))
    }

    /// 解析一行一条的表：`词\t条目`，整个第二列是一条（颜文字里有空格）；同一个词的多条按行序排。
    pub fn parse_entries(source: &str) -> Result<Self, usize> {
        let mut lists = Lists::new();
        for (index, line) in source.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let (word, item) = line.split_once('\t').ok_or(index + 1)?;
            let list = lists.entry(word.to_owned()).or_default();
            if !list.iter().any(|e| e == item) {
                list.push(item.to_owned());
            }
        }
        Ok(Self::pack(lists))
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let source = std::fs::read_to_string(path)?;
        Self::parse(&source).map_err(|line| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("emoji table line {line}: malformed entry"),
            )
        })
    }

    fn pack(lists: Lists) -> Self {
        let items: usize = lists.values().map(Vec::len).sum();
        let mut text = String::with_capacity(lists.values().flatten().map(String::len).sum());
        let mut bounds = Vec::with_capacity(items + 1);
        bounds.push(0);
        let mut words = HashMap::with_capacity(lists.len());
        for (word, list) in lists {
            let start = (bounds.len() - 1) as u32;
            for item in list {
                text.push_str(&item);
                bounds.push(text.len() as u32);
            }
            words.insert(word.into_boxed_str(), (start, (bounds.len() - 1) as u32));
        }
        Self {
            words,
            text,
            bounds,
        }
    }

    fn unpack(self) -> Lists {
        let mut lists = Lists::with_capacity(self.words.len());
        for (word, &(start, end)) in &self.words {
            lists.insert(
                word.to_string(),
                (start..end).map(|i| self.item(i).to_owned()).collect(),
            );
        }
        lists
    }

    pub(super) fn item(&self, index: u32) -> &str {
        let index = index as usize;
        &self.text[self.bounds[index] as usize..self.bounds[index + 1] as usize]
    }

    /// 并入另一张表（中文表 + 英文表合成一张：键是不同文字，不会撞）；同键的 emoji 追加去重。
    pub fn merge(&mut self, other: EmojiTable) {
        let mut lists = std::mem::take(self).unpack();
        for (word, emojis) in other.unpack() {
            let list = lists.entry(word).or_default();
            for emoji in emojis {
                if !list.contains(&emoji) {
                    list.push(emoji);
                }
            }
        }
        *self = Self::pack(lists);
    }

    /// 只留 `keep` 认的条目，删空的词。
    pub fn retain(&mut self, mut keep: impl FnMut(&str) -> bool) {
        let mut lists = std::mem::take(self).unpack();
        lists.retain(|_, list| {
            list.retain(|item| keep(item));
            !list.is_empty()
        });
        *self = Self::pack(lists);
    }

    /// 这个词对应的条目（没有就是空）。
    pub fn lookup(&self, word: &str) -> Items<'_> {
        let (start, end) = self.words.get(word).copied().unwrap_or((0, 0));
        Items::new(self, start, end)
    }

    #[cfg(test)]
    pub(crate) fn words(&self) -> impl Iterator<Item = &str> {
        self.words.keys().map(|word| &**word)
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_deduplicates() {
        let table = EmojiTable::parse("# 注释\n笑\t😄 😆\n笑\t😆 🤣\n").unwrap();
        assert!(table.lookup("笑").eq(["😄", "😆", "🤣"]));
        assert_eq!(table.lookup("哭").len(), 0);
        assert_eq!(EmojiTable::parse("坏行\n").unwrap_err(), 1);
    }

    #[test]
    fn entries_keep_spaces_and_line_order() {
        let mut table =
            EmojiTable::parse_entries("开心\t( ^▽^ )\n开心\t(＾▽＾)\n哭\t(T_T)\n").unwrap();
        assert!(table.lookup("开心").eq(["( ^▽^ )", "(＾▽＾)"]));
        table.retain(|item| item != "(T_T)");
        assert_eq!(table.lookup("哭").len(), 0);
        assert_eq!(table.len(), 1);
        assert_eq!(EmojiTable::parse_entries("坏行\n").unwrap_err(), 1);
    }

    #[test]
    fn merge_appends_without_duplicates() {
        let mut zh = EmojiTable::parse("笑\t😄 😀\n").unwrap();
        let en = EmojiTable::parse("smile\t😀\n笑\t😀 😁\n").unwrap();
        zh.merge(en);
        assert!(zh.lookup("smile").eq(["😀"]));
        assert!(zh.lookup("笑").eq(["😄", "😀", "😁"]));
        assert_eq!(zh.len(), 2);
    }
}
