//! 符号 / 表情面板的内容：Core 编进程序的面板目录原样交给壳，壳按分类排格子，点了直接上屏。

mod section;

pub use section::CatalogSection;

use std::path::Path;

use qingjian_core::{PanelGroup, catalog_sections, common_items};
use qingjian_learning::PanelUsage;

/// 全部分类，按面板里的顺序。
pub fn catalog() -> Vec<CatalogSection> {
    catalog_sections()
        .iter()
        .map(|section| CatalogSection {
            group: section.group.label(),
            name: &section.name,
            items: section
                .items
                .iter()
                .map(|item| item.text.as_str())
                .collect(),
        })
        .collect()
}

pub(crate) fn catalog_json() -> String {
    serde_json::to_string(&catalog()).expect("面板目录总能序列化")
}

/// 「符号」或「表情」组的「常用」该显示的条目：用户用过的在前（见 [`common_items`]）；组名不认识返回空。
pub fn common(user_dir: &Path, group: &str) -> Vec<String> {
    let group = match group {
        "符号" => PanelGroup::Symbols,
        "表情" => PanelGroup::Emoji,
        _ => return Vec::new(),
    };
    common_items(group, PanelUsage::open(user_dir).uses())
}

/// 面板里点了一格：记一次，下次打开面板时排进「常用」。
pub fn record_use(user_dir: &Path, item: &str) {
    if let Err(error) = PanelUsage::open(user_dir).record(item) {
        tracing::warn!(%error, "面板使用记录写不进去");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_symbols_then_emoji() {
        let sections = catalog();
        assert_eq!(sections.first().map(|s| s.group), Some("符号"));
        assert!(
            sections
                .iter()
                .any(|s| s.group == "表情" && s.items.contains(&"😂"))
        );
        assert!(sections.iter().all(|s| !s.items.is_empty()));
    }
}
