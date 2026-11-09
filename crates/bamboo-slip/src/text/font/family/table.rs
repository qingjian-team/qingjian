//! 字族表：用到的回退链去重后排成一张表，文字样式里只存序号（[`FamilyId`]），样式因此仍可按值复制。
//! 使用方建一次交给 [`crate::TextPainter::use_families`]，画笔按序号查链、挑第一个装了的字族。

use super::{FamilyId, FamilyList};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyTable {
    /// 回退链，序号就是 [`FamilyId`]；第 0 条是缺省字族。
    chains: Vec<FamilyList>,
}

impl FamilyTable {
    /// 只有缺省字族的表。
    pub fn new(default: FamilyList) -> Self {
        Self {
            chains: vec![default],
        }
    }

    /// 这条链的序号；表里没有就加在末尾。
    pub fn intern(&mut self, chain: &FamilyList) -> FamilyId {
        let index = match self.chains.iter().position(|known| known == chain) {
            Some(index) => index,
            None => {
                self.chains.push(chain.clone());
                self.chains.len() - 1
            }
        };
        FamilyId(u16::try_from(index).unwrap_or(0))
    }

    /// 全部回退链，按序号排。
    pub fn chains(&self) -> &[FamilyList] {
        &self.chains
    }

    /// 链里写到的字族名（不含 `system`），去重；使用方按名字查字体文件交给画笔加载。
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for name in self.chains.iter().flat_map(|chain| &chain.0) {
            if !FamilyList::is_system(name) && !names.contains(name) {
                names.push(name.clone());
            }
        }
        names
    }
}

impl Default for FamilyTable {
    fn default() -> Self {
        Self::new(FamilyList::system())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain(names: &[&str]) -> FamilyList {
        FamilyList(names.iter().map(|name| (*name).to_owned()).collect())
    }

    #[test]
    fn same_chain_shares_one_id() {
        let mut table = FamilyTable::default();
        let songti = table.intern(&chain(&["Songti SC", "system"]));
        assert_eq!(songti, FamilyId(1));
        assert_eq!(table.intern(&chain(&["Songti SC", "system"])), songti);
        assert_eq!(table.intern(&FamilyList::system()), FamilyId(0));
        assert_eq!(table.names(), ["Songti SC"]);
    }
}
