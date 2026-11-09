//! 打开着的面板：搜索结果（有的话）加内置分类，记着当前分类与每组上次停在哪个分类。
//! 符号组与表情组的「常用」打开时按用户用过的重排（[`common_items`]），其余分类原样用内置的。
//! 翻译组排最前，每种目标语言算一个分类（没有条目），框里的原文与译文在 [`TranslateDraft`]。

use std::collections::HashMap;

use super::common::COMMON;
use super::translate::{TARGET_LANGUAGES, TranslateDraft, TranslateStatus};
use super::{CATALOG, PanelGroup, PanelSection, PanelUse, common_items};
use crate::candidate::{Candidate, CandidateKind, Language, learning_languages};
use crate::i18n::{tr, tr_args};

#[derive(Debug, Clone)]
pub struct SymbolPanel {
    /// 翻译组：[`TARGET_LANGUAGES`] 每种一个分类；翻译不可用时为空。
    translate: Vec<PanelSection>,

    /// 翻译框里的原文与翻译进度。
    draft: TranslateDraft,

    /// 搜索结果组，没搜到东西时为 `None`。
    search: Option<PanelSection>,

    /// 按用户用过的重排的「常用」分类，顶替内置目录里同组同名的那个。
    common: Vec<PanelSection>,

    /// 当前分类在 [`Self::sections`] 里的下标。
    current: usize,

    /// 每组上次停在哪个分类，切回这组时回到那里。
    last: HashMap<PanelGroup, usize>,
}

impl SymbolPanel {
    /// 有搜索结果就停在结果上，否则停在符号组的第一个分类（常用）。`uses` 是面板条目的使用记录。
    pub(crate) fn new(query: &str, results: Vec<Candidate>, uses: &[PanelUse]) -> Self {
        let search = (!results.is_empty()).then(|| PanelSection {
            group: PanelGroup::Search,
            name: query.to_owned(),
            items: results,
        });
        Self {
            translate: Vec::new(),
            draft: TranslateDraft::default(),
            search,
            common: ranked_common(uses),
            current: 0,
            last: HashMap::new(),
        }
    }

    /// 加上翻译组，目标语言缺省是 `chosen`；还停在原来那组。
    pub(crate) fn with_translate(mut self, chosen: Language) -> Self {
        let index = TARGET_LANGUAGES
            .iter()
            .position(|l| *l == chosen)
            .unwrap_or(0);
        self.translate = TARGET_LANGUAGES
            .iter()
            .map(|language| PanelSection {
                group: PanelGroup::Translate,
                name: language_name(*language).to_owned(),
                items: Vec::new(),
            })
            .collect();
        self.current += self.translate.len();
        self.last.insert(PanelGroup::Translate, index);
        self
    }

    /// 换到 `group`（落在那组上次停的分类）；面板里没有这组就不动。返回停没停在那组。
    pub(crate) fn open_on(&mut self, group: PanelGroup) -> bool {
        if self.group() == group {
            return true;
        }
        let Some(index) = self.groups().iter().position(|g| *g == group) else {
            return false;
        };
        self.select_group(index)
    }

    /// 翻译组选的目标语言：在翻译组就是当前分类，不在就是上次停的；没有翻译组时为 `None`。
    pub fn target_language(&self) -> Option<Language> {
        if self.translate.is_empty() {
            return None;
        }
        let index = if self.group() == PanelGroup::Translate {
            self.current
        } else {
            self.last.get(&PanelGroup::Translate).copied().unwrap_or(0)
        };
        TARGET_LANGUAGES.get(index).copied()
    }

    /// 翻译框里的原文（不含还在打的拼音）。
    pub fn translate_source(&self) -> &str {
        self.draft.source()
    }

    /// 光标在原文里的位置（第几个字之后）；还在打的拼音画在这里，选的词插在这里。
    pub fn translate_cursor(&self) -> usize {
        self.draft.cursor()
    }

    /// 原文里选中的字（按字算）；没选中东西时为 `None`。
    pub fn translate_selection(&self) -> Option<std::ops::Range<usize>> {
        self.draft.selection()
    }

    /// 最近一条译文，原文改了还没译好时也先留着，这次没翻出来就没有；壳把它放进应用的 preedit。
    pub fn translation(&self) -> Option<&str> {
        self.draft.translation(self.target_language()?)
    }

    pub fn translate_status(&self) -> TranslateStatus {
        self.target_language()
            .map_or(TranslateStatus::Idle, |language| {
                self.draft.status(language)
            })
    }

    /// 翻译框底下那一行：实际译成什么语言（外文原文是中文）与进度，随界面语言。
    pub fn translate_line(&self) -> String {
        let Some(chosen) = self.target_language() else {
            return String::new();
        };
        let language = self.draft.job(chosen).map_or(chosen, |job| job.language);
        let name = match language {
            Language::Chinese => tr("panel.chinese"),
            language => language_name(language),
        };
        let state = match self.translate_status() {
            TranslateStatus::Idle => tr("panel.translate_idle"),
            TranslateStatus::Waiting => tr("panel.translate_waiting"),
            TranslateStatus::Ready => tr("panel.translate_ready"),
            TranslateStatus::Failed => tr("panel.translate_failed"),
        };
        format!(
            "{} · {state}",
            tr_args("panel.translate_to", &[("language", &name)])
        )
    }

    /// 框里没字时的提示，随界面语言。
    pub fn translate_placeholder(&self) -> &'static str {
        tr("panel.translate_placeholder")
    }

    pub(crate) fn draft(&self) -> &TranslateDraft {
        &self.draft
    }

    pub(crate) fn draft_mut(&mut self) -> &mut TranslateDraft {
        &mut self.draft
    }

    /// 全部分类：翻译组在最前，然后是搜索结果，再是内置的（「常用」换成重排过的）。
    fn sections(&self) -> impl Iterator<Item = &PanelSection> {
        self.translate
            .iter()
            .chain(self.search.iter())
            .chain(CATALOG.sections.iter().map(|builtin| {
                self.common
                    .iter()
                    .find(|s| s.group == builtin.group && s.name == builtin.name)
                    .unwrap_or(builtin)
            }))
    }

    fn len(&self) -> usize {
        self.translate.len() + usize::from(self.search.is_some()) + CATALOG.sections.len()
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

/// 语言在界面上的名字（随界面语言）。
fn language_name(language: Language) -> &'static str {
    learning_languages()
        .into_iter()
        .find(|(code, _)| *code == language.code())
        .map_or(language.code(), |(_, name)| name)
}

/// 符号组与表情组的「常用」按 `uses` 重排；目录里没有的条目（带肤色的 emoji）现做一个候选。
fn ranked_common(uses: &[PanelUse]) -> Vec<PanelSection> {
    CATALOG
        .sections
        .iter()
        .filter(|section| section.name == COMMON)
        .map(|section| PanelSection {
            group: section.group,
            name: section.name.clone(),
            items: common_items(section.group, uses)
                .into_iter()
                .map(|text| {
                    CATALOG.item(&text).cloned().unwrap_or(Candidate {
                        text,
                        kind: CandidateKind::Emoji,
                        syllables: Vec::new(),
                        reading: None,
                        translation: None,
                        aux_code: None,
                    })
                })
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut panel = SymbolPanel::new("jt", vec![item("→")], &[]);
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
        let panel = SymbolPanel::new("x", Vec::new(), &[]);
        assert_eq!(panel.group(), PanelGroup::Symbols);
        assert_eq!(panel.groups(), [PanelGroup::Symbols, PanelGroup::Emoji]);
    }

    #[test]
    fn translate_group_comes_first_and_its_sections_are_languages() {
        let mut panel =
            SymbolPanel::new("nihao", vec![item("👋")], &[]).with_translate(Language::Japanese);
        assert_eq!(panel.group(), PanelGroup::Search);
        assert_eq!(panel.target_language(), Some(Language::Japanese));
        assert!(panel.open_on(PanelGroup::Translate));
        assert_eq!(panel.groups()[0], PanelGroup::Translate);
        assert_eq!(panel.target_language(), Some(Language::Japanese));
        assert!(panel.switch_section(1));
        assert_eq!(panel.target_language(), Some(Language::Spanish));
        // 去别的组再回来，语言还是刚才换的
        assert!(panel.switch_group(1));
        assert_eq!(panel.target_language(), Some(Language::Spanish));
        assert!(panel.open_on(PanelGroup::Translate));
        assert_eq!(panel.group_sections().1, 2);
    }

    #[test]
    fn common_sections_put_used_items_first() {
        let uses = [
            PanelUse {
                item: "★".to_owned(),
                count: 3,
                last: 1,
            },
            PanelUse {
                item: "👍🏽".to_owned(),
                count: 1,
                last: 2,
            },
        ];
        let mut panel = SymbolPanel::new("", Vec::new(), &uses);
        assert_eq!(panel.section().name, "常用");
        assert_eq!(panel.section().items[0].text, "★");
        assert_eq!(panel.section().items[0].kind, CandidateKind::Symbol);
        assert!(panel.switch_group(1));
        assert_eq!(panel.group(), PanelGroup::Emoji);
        assert_eq!(panel.section().name, "常用");
        assert_eq!(panel.section().items[0].text, "👍🏽");
        assert_eq!(panel.section().items[0].kind, CandidateKind::Emoji);
        // 其余分类照旧是内置的
        assert!(panel.switch_section(1));
        assert_ne!(panel.section().name, "常用");
    }
}
