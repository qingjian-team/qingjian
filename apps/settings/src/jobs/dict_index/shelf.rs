//! 「可下载的」那一节的两个标签：词库与释义表，一次只列一种。

use crate::jobs::dict_index::DictEntry;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Shelf {
    #[default]
    Dictionaries,

    Glossaries,
}

impl Shelf {
    pub const ALL: [Self; 2] = [Self::Dictionaries, Self::Glossaries];

    pub fn holds(self, entry: &DictEntry) -> bool {
        (self == Self::Glossaries) == entry.glossary_language().is_some()
    }
}
