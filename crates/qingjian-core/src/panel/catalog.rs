//! 编进程序的面板数据：`catalog.tsv`（分组\t分类\t条目\t名字）按行序排出分类与条目，`keywords.tsv`（词\t条目）是搜索用的关键词。

use std::collections::HashMap;
use std::sync::LazyLock;

use super::{PanelGroup, PanelSection};
use crate::candidate::{Candidate, CandidateKind};
use crate::emoji::{EmojiTable, is_emoji};

/// 颜文字所在的分类名：里面有 `♥` 这类带 emoji 属性的字符，不能靠字符认种类。
const KAOMOJI_SECTION: &str = "颜文字";

pub(crate) struct Catalog {
    pub sections: Vec<PanelSection>,

    /// 条目 → 它在面板里的样子（种类与名字），搜索结果按它补全。
    index: HashMap<String, Candidate>,
}

impl Catalog {
    fn parse(source: &str) -> Self {
        let mut sections: Vec<PanelSection> = Vec::new();
        let mut index = HashMap::new();
        for (number, line) in source.lines().enumerate() {
            let mut fields = line.split('\t');
            let (Some(group), Some(name), Some(text), name_field) =
                (fields.next(), fields.next(), fields.next(), fields.next())
            else {
                panic!(
                    "built-in panel catalog line {}: malformed entry",
                    number + 1
                );
            };
            let group = PanelGroup::from_label(group).unwrap_or_else(|| {
                panic!("built-in panel catalog line {}: unknown group", number + 1)
            });
            let kind = if name == KAOMOJI_SECTION {
                CandidateKind::Kaomoji
            } else if is_emoji(text) {
                CandidateKind::Emoji
            } else {
                CandidateKind::Symbol
            };
            let item = Candidate {
                text: text.to_owned(),
                kind,
                syllables: Vec::new(),
                reading: name_field.filter(|n| !n.is_empty()).map(str::to_owned),
                translation: None,
                aux_code: None,
            };
            index
                .entry(item.text.clone())
                .or_insert_with(|| item.clone());
            match sections.last_mut() {
                Some(last) if last.group == group && last.name == name => last.items.push(item),
                _ => sections.push(PanelSection {
                    group,
                    name: name.to_owned(),
                    items: vec![item],
                }),
            }
        }
        Self { sections, index }
    }

    /// 面板里的这一条；不在面板里的返回 `None`。
    pub fn item(&self, text: &str) -> Option<&Candidate> {
        self.index.get(text)
    }
}

pub(crate) static CATALOG: LazyLock<Catalog> =
    LazyLock::new(|| Catalog::parse(include_str!("../../data/panel/catalog.tsv")));

/// CLDR 中文关键词与符号库检索词：词 → 条目。
pub(crate) static KEYWORDS: LazyLock<EmojiTable> = LazyLock::new(|| {
    EmojiTable::parse_entries(include_str!("../../data/panel/keywords.tsv"))
        .unwrap_or_else(|line| panic!("built-in panel keywords line {line}: malformed entry"))
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_both_groups_and_names() {
        let groups: Vec<_> = CATALOG.sections.iter().map(|s| s.group).collect();
        assert_eq!(groups.first(), Some(&PanelGroup::Symbols));
        assert!(groups.contains(&PanelGroup::Emoji));
        assert!(CATALOG.sections.iter().all(|s| !s.items.is_empty()));
        assert_eq!(CATALOG.item("≈").unwrap().kind, CandidateKind::Symbol);
        assert_eq!(
            CATALOG.item("😂").unwrap().reading.as_deref(),
            Some("笑哭了")
        );
        assert_eq!(CATALOG.item("(T_T)").unwrap().kind, CandidateKind::Kaomoji);
        assert!(KEYWORDS.lookup("箭头").contains(&"→".to_owned()));
    }
}
