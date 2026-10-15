//! 打开着的面板：搜索结果（有的话）加内置分类，记着当前分类与每组上次停在哪个分类。

use std::collections::HashMap;

use super::{CATALOG, PanelGroup, PanelSection};
use crate::candidate::Candidate;

#[derive(Debug, Clone)]
pub struct SymbolPanel {
    /// 搜索结果组，没搜到东西时为 `None`。
    search: Option<PanelSection>,

    /// 当前分类在 [`Self::sections`] 里的下标。
    current: usize,

    /// 每组上次停在哪个分类，切回这组时回到那里。
    last: HashMap<PanelGroup, usize>,
}

impl SymbolPanel {
    /// 有搜索结果就停在结果上，否则停在符号组的第一个分类（常用）。
    pub(crate) fn new(query: &str, results: Vec<Candidate>) -> Self {
        let search = (!results.is_empty()).then(|| PanelSection {
            group: PanelGroup::Search,
            name: query.to_owned(),
            items: results,
        });
        Self {
            search,
            current: 0,
            last: HashMap::new(),
        }
    }

    /// 全部分类：搜索结果在前，然后是内置的。
    fn sections(&self) -> impl Iterator<Item = &PanelSection> {
        self.search.iter().chain(CATALOG.sections.iter())
    }

    fn len(&self) -> usize {
        usize::from(self.search.is_some()) + CATALOG.sections.len()
    }

    pub fn section(&self) -> &PanelSection {
        self.sections()
            .nth(self.current)
            .expect("current 总在分类范围内")
    }

    pub fn group(&self) -> PanelGroup {
        self.section().group
    }

    /// 有哪几组，按顺序。
    pub fn groups(&self) -> Vec<PanelGroup> {
        let mut groups: Vec<PanelGroup> = Vec::new();
        for section in self.sections() {
            if groups.last() != Some(&section.group) {
                groups.push(section.group);
            }
        }
        groups
    }

    /// 当前组的分类名与当前分类在其中的位置。
    pub fn group_sections(&self) -> (Vec<&str>, usize) {
        let group = self.group();
        let names: Vec<&str> = self
            .sections()
            .filter(|s| s.group == group)
            .map(|s| s.name.as_str())
            .collect();
        let first = self.first_of(group);
        (names, self.current - first)
    }

    fn first_of(&self, group: PanelGroup) -> usize {
        self.sections().position(|s| s.group == group).unwrap_or(0)
    }

    /// 在当前组里换分类，到头绕回来。返回换没换。
    pub fn switch_section(&mut self, delta: isize) -> bool {
        let group = self.group();
        let first = self.first_of(group);
        let count = self.sections().filter(|s| s.group == group).count();
        if count <= 1 {
            return false;
        }
        let offset = (self.current - first) as isize + delta;
        self.current = first + offset.rem_euclid(count as isize) as usize;
        true
    }

    /// 换到下一组 / 上一组（绕回），落在那组上次停的分类。返回换没换。
    pub fn switch_group(&mut self, delta: isize) -> bool {
        let groups = self.groups();
        if groups.len() <= 1 {
            return false;
        }
        let here = self.group();
        let position = groups.iter().position(|g| *g == here).unwrap_or(0) as isize;
        let target = groups[(position + delta).rem_euclid(groups.len() as isize) as usize];
        self.last.insert(here, self.current);
        let first = self.first_of(target);
        self.current = self
            .last
            .get(&target)
            .copied()
            .filter(|&i| i < self.len())
            .unwrap_or(first);
        true
    }

    /// 直接换到第 `index` 组（[`Self::groups`] 的顺序，点胶囊用）。返回换没换。
    pub fn select_group(&mut self, index: usize) -> bool {
        let groups = self.groups();
        let Some(position) = groups.iter().position(|g| *g == self.group()) else {
            return false;
        };
        if index >= groups.len() || index == position {
            return false;
        }
        self.switch_group(index as isize - position as isize)
    }

    /// 直接换到当前组的第 `index` 个分类（点标签用）。返回换没换。
    pub fn select_section(&mut self, index: usize) -> bool {
        let (names, position) = self.group_sections();
        if index >= names.len() || index == position {
            return false;
        }
        self.switch_section(index as isize - position as isize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidate::CandidateKind;

    fn item(text: &str) -> Candidate {
        Candidate {
            text: text.to_owned(),
            kind: CandidateKind::Symbol,
            syllables: Vec::new(),
            reading: None,
            translation: None,
            aux_code: None,
        }
    }

    #[test]
    fn opens_on_results_and_cycles_sections_within_a_group() {
        let mut panel = SymbolPanel::new("jt", vec![item("→")]);
        assert_eq!(panel.group(), PanelGroup::Search);
        assert_eq!(
            panel.groups(),
            [PanelGroup::Search, PanelGroup::Symbols, PanelGroup::Emoji]
        );
        // 搜索组只有一个分类，换分类不动
        assert!(!panel.switch_section(1));
        assert!(panel.switch_group(1));
        assert_eq!(panel.group(), PanelGroup::Symbols);
        assert_eq!(panel.section().name, "常用");
        assert!(panel.switch_section(-1));
        let (names, index) = panel.group_sections();
        assert_eq!(index, names.len() - 1);
        // 切去表情再切回来，回到符号组上次停的分类
        let last = panel.section().name.clone();
        assert!(panel.switch_group(1));
        assert_eq!(panel.group(), PanelGroup::Emoji);
        assert!(panel.switch_group(-1));
        assert_eq!(panel.section().name, last);
        // 点胶囊、点标签直接跳
        assert!(panel.select_group(0));
        assert_eq!(panel.group(), PanelGroup::Search);
        assert!(panel.select_group(1));
        assert!(panel.select_section(2));
        assert_eq!(panel.group_sections().1, 2);
        assert!(!panel.select_section(2));
    }

    #[test]
    fn without_results_opens_on_common_symbols() {
        let panel = SymbolPanel::new("x", Vec::new());
        assert_eq!(panel.group(), PanelGroup::Symbols);
        assert_eq!(panel.groups(), [PanelGroup::Symbols, PanelGroup::Emoji]);
    }
}
