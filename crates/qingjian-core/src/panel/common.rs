//! 「常用」分类按用户自己用的排：用过的按次数在前，后面接内置的常用表，去重。各端面板共用。

use super::section::{NARROW_EMS, item_ems};
use super::{CATALOG, PanelGroup, PanelUse};
use crate::candidate::{Candidate, CandidateKind};
use crate::emoji::is_emoji;

/// 名为「常用」的那一类。
pub(super) const COMMON: &str = "常用";

/// `group` 组「常用」分类该显示的条目：`uses` 里属于这一组的按次数、再按最近排在前面，
/// 后面接内置常用表里剩下的。宽的条目（`.com`、颜文字）不提进来，免得整类格子变宽。
pub fn common_items(group: PanelGroup, uses: &[PanelUse]) -> Vec<String> {
    let mut used: Vec<&PanelUse> = uses
        .iter()
        .filter(|entry| entry.count > 0 && belongs(group, &entry.item))
        .collect();
    used.sort_by(|a, b| b.count.cmp(&a.count).then(b.last.cmp(&a.last)));
    let mut items: Vec<String> = used.iter().map(|entry| entry.item.clone()).collect();
    if let Some(section) = CATALOG
        .sections
        .iter()
        .find(|section| section.group == group && section.name == COMMON)
    {
        for item in &section.items {
            if !items.contains(&item.text) {
                items.push(item.text.clone());
            }
        }
    }
    items
}

/// 条目算不算这一组、能不能进它的「常用」：符号要在面板目录里、种类是符号；
/// emoji 不在目录里也算（长按选的带肤色的那条）；都要一格放得下。
fn belongs(group: PanelGroup, text: &str) -> bool {
    let kind = match CATALOG.item(text) {
        Some(item) => item.kind,
        None if group == PanelGroup::Emoji && is_emoji(text) => CandidateKind::Emoji,
        None => return false,
    };
    let wanted = match group {
        PanelGroup::Symbols => CandidateKind::Symbol,
        PanelGroup::Emoji => CandidateKind::Emoji,
        PanelGroup::Search => return false,
    };
    let probe = Candidate {
        text: text.to_owned(),
        kind,
        syllables: Vec::new(),
        reading: None,
        translation: None,
        aux_code: None,
    };
    kind == wanted && item_ems(&probe) <= NARROW_EMS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn used(item: &str, count: u32, last: i64) -> PanelUse {
        PanelUse {
            item: item.to_owned(),
            count,
            last,
        }
    }

    #[test]
    fn used_symbols_come_first_by_count_then_recency() {
        let uses = [used("→", 2, 10), used("★", 5, 1), used("√", 2, 20)];
        let items = common_items(PanelGroup::Symbols, &uses);
        assert_eq!(&items[..3], ["★", "√", "→"]);
        assert_eq!(items.iter().filter(|s| *s == "★").count(), 1);
    }

    #[test]
    fn keeps_groups_and_wide_items_apart() {
        let uses = [used("😂", 3, 1), used("http://", 9, 1), used("，", 1, 1)];
        let symbols = common_items(PanelGroup::Symbols, &uses);
        assert_eq!(symbols[0], "，");
        assert!(!symbols.iter().any(|s| s == "😂" || s == "http://"));
        let emoji = common_items(PanelGroup::Emoji, &uses);
        assert_eq!(emoji[0], "😂");
    }

    #[test]
    fn skin_toned_emoji_count_even_though_not_in_catalog() {
        let uses = [used("👍🏽", 1, 1)];
        assert_eq!(common_items(PanelGroup::Emoji, &uses)[0], "👍🏽");
    }

    #[test]
    fn without_use_it_is_the_builtin_list() {
        let items = common_items(PanelGroup::Symbols, &[]);
        assert!(!items.is_empty());
        assert_eq!(
            items.first(),
            CATALOG
                .sections
                .iter()
                .find(|s| s.group == PanelGroup::Symbols && s.name == COMMON)
                .and_then(|s| s.items.first())
                .map(|c| &c.text)
        );
    }
}
