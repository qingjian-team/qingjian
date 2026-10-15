use std::collections::HashMap;
use std::path::Path;

/// 词 → emoji 列表。表里一个词后面的 emoji 按相关度排好（名字正好是这个词的在前）。
#[derive(Debug, Default)]
pub struct EmojiTable {
    /// 词 → emoji。
    entries: HashMap<String, Vec<String>>,
}

impl EmojiTable {
    /// 解析 TSV：`词\temoji emoji …`，空行与 `#` 开头跳过；格式不对的行返回行号。
    pub fn parse(source: &str) -> Result<Self, usize> {
        let mut entries: HashMap<String, Vec<String>> = HashMap::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (word, emojis) = line.split_once('\t').ok_or(index + 1)?;
            let list = entries.entry(word.to_owned()).or_default();
            for emoji in emojis.split(' ').filter(|e| !e.is_empty()) {
                if !list.iter().any(|e| e == emoji) {
                    list.push(emoji.to_owned());
                }
            }
        }
        Ok(Self { entries })
    }

    /// 解析一行一条的表：`词\t条目`，整个第二列是一条（颜文字里有空格）；同一个词的多条按行序排。
    pub fn parse_entries(source: &str) -> Result<Self, usize> {
        let mut entries: HashMap<String, Vec<String>> = HashMap::new();
        for (index, line) in source.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let (word, item) = line.split_once('\t').ok_or(index + 1)?;
            let list = entries.entry(word.to_owned()).or_default();
            if !list.iter().any(|e| e == item) {
                list.push(item.to_owned());
            }
        }
        Ok(Self { entries })
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

    /// 这个词对应的 emoji（没有就是空）。
    /// 并入另一张表（中文表 + 英文表合成一张：键是不同文字，不会撞）；同键的 emoji 追加去重。
    pub fn merge(&mut self, other: EmojiTable) {
        for (word, emojis) in other.entries {
            let list = self.entries.entry(word).or_default();
            for emoji in emojis {
                if !list.contains(&emoji) {
                    list.push(emoji);
                }
            }
        }
    }

    /// 只留 `keep` 认的条目，删空的词。
    pub fn retain(&mut self, mut keep: impl FnMut(&str) -> bool) {
        self.entries.retain(|_, list| {
            list.retain(|item| keep(item));
            !list.is_empty()
        });
    }

    pub fn lookup(&self, word: &str) -> &[String] {
        self.entries.get(word).map_or(&[], Vec::as_slice)
    }

    #[cfg(test)]
    pub(crate) fn words(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_deduplicates() {
        let table = EmojiTable::parse("# 注释\n笑\t😄 😆\n笑\t😆 🤣\n").unwrap();
        assert_eq!(table.lookup("笑"), ["😄", "😆", "🤣"]);
        assert!(table.lookup("哭").is_empty());
        assert_eq!(EmojiTable::parse("坏行\n").unwrap_err(), 1);
    }

    #[test]
    fn entries_keep_spaces_and_line_order() {
        let mut table =
            EmojiTable::parse_entries("开心\t( ^▽^ )\n开心\t(＾▽＾)\n哭\t(T_T)\n").unwrap();
        assert_eq!(table.lookup("开心"), ["( ^▽^ )", "(＾▽＾)"]);
        table.retain(|item| item != "(T_T)");
        assert!(table.lookup("哭").is_empty());
        assert_eq!(table.len(), 1);
        assert_eq!(EmojiTable::parse_entries("坏行\n").unwrap_err(), 1);
    }

    #[test]
    fn merge_appends_without_duplicates() {
        let mut zh = EmojiTable::parse("笑\t😄 😀\n").unwrap();
        let en = EmojiTable::parse("smile\t😀\n笑\t😀 😁\n").unwrap();
        zh.merge(en);
        assert_eq!(zh.lookup("smile"), ["😀"]);
        assert_eq!(zh.lookup("笑"), ["😄", "😀", "😁"]);
        assert_eq!(zh.len(), 2);
    }
}
