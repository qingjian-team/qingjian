//! 主题的字族表：缺省字族与各命名样式的回退链去重后排成一张表，样式里存序号（[`FamilyId`]），文字样式因此仍可按值复制。
//! 加载主题时建一次；渲染器按序号查链、挑第一个装了的字族。

use std::collections::HashMap;

use super::{FamilyId, FontSpec};
use crate::theme::file::{FamilyList, FontStyle, TextSettings};

#[derive(Debug, Default)]
pub(crate) struct FontFamilies {
    /// 回退链，序号就是 [`FamilyId`]；第 0 条是缺省字族。
    chains: Vec<FamilyList>,

    /// 命名样式（点），字族已换成序号。
    styles: HashMap<String, FontSpec>,
}

impl FontFamilies {
    pub(crate) fn new(text: &TextSettings) -> Self {
        let mut chains = vec![text.family.clone().unwrap_or_else(FamilyList::system)];
        // 按样式名排，字族表的顺序（也是壳查字体的顺序）每次加载都一样
        let mut named: Vec<_> = text.styles.iter().collect();
        named.sort_by(|a, b| a.0.cmp(b.0));
        let styles = named
            .into_iter()
            .map(|(name, spec)| {
                let family = match &spec.family {
                    Some(list) => intern(&mut chains, list),
                    None => FamilyId(0),
                };
                let font = FontSpec {
                    size: spec.size,
                    line_height: spec.line_height,
                    weight: spec.weight,
                    italic: spec.style == FontStyle::Italic,
                    family,
                };
                (name.clone(), font)
            })
            .collect();
        Self { chains, styles }
    }

    pub(crate) fn style(&self, name: &str) -> Option<FontSpec> {
        self.styles.get(name).copied()
    }

    /// 全部回退链，按序号排。
    pub(crate) fn chains(&self) -> &[FamilyList] {
        &self.chains
    }

    /// 链里写到的字族名（不含 `system`），去重；壳按名字查字体文件交给渲染器加载。
    pub(crate) fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for name in self.chains.iter().flat_map(|chain| &chain.0) {
            if !FamilyList::is_system(name) && !names.contains(name) {
                names.push(name.clone());
            }
        }
        names
    }
}

fn intern(chains: &mut Vec<FamilyList>, list: &FamilyList) -> FamilyId {
    let index = chains
        .iter()
        .position(|chain| chain == list)
        .unwrap_or_else(|| {
            chains.push(list.clone());
            chains.len() - 1
        });
    FamilyId(u16::try_from(index).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use crate::theme::Theme;
    use crate::theme::file::FontRef;

    fn theme(text: &str) -> Theme {
        let json = format!(
            r#"{{ "extends": "qingjian", "schema": 1, "meta": {{ "id": "t", "name": "t" }}, "text": {text} }}"#
        );
        Theme::from_json(&json, false).unwrap()
    }

    #[test]
    fn styles_without_family_use_theme_default() {
        let theme = theme(r#"{ "family": ["Songti SC", "system"] }"#);
        let candidate = theme.font("candidate");
        assert_eq!(candidate.family.0, 0);
        assert_eq!(
            theme.families().chains()[0].0,
            ["Songti SC".to_owned(), "system".to_owned()]
        );
        assert_eq!(theme.font_families(), ["Songti SC"]);
    }

    #[test]
    fn same_chain_shares_one_id_and_italic_parses() {
        let theme = theme(
            r#"{ "styles": {
                "candidate": { "size": 16, "line_height": 19, "family": "Songti SC" },
                "annotation": { "size": 12, "line_height": 15, "family": ["Songti SC"] },
                "index": { "size": 12, "line_height": 15, "family": "Georgia", "style": "italic" }
            } }"#,
        );
        let (candidate, annotation, index) = (
            theme.font("candidate"),
            theme.font("annotation"),
            theme.font("index"),
        );
        assert_eq!(candidate.family, annotation.family);
        assert_ne!(candidate.family, index.family);
        assert!(index.italic && !candidate.italic);
        assert_eq!(theme.font_families(), ["Songti SC", "Georgia"]);
    }

    #[test]
    fn inline_font_keeps_family_and_can_switch_style() {
        let theme = theme(
            r#"{ "styles": { "candidate": { "size": 16, "line_height": 19, "family": "Georgia" } } }"#,
        );
        let font: FontRef =
            serde_json::from_str(r#"{ "base": "candidate", "size": 20, "style": "italic" }"#)
                .unwrap();
        let spec = theme.font_ref(&font);
        assert_eq!(spec.family, theme.font("candidate").family);
        assert!(spec.italic);
        assert_eq!(spec.size, 20.0);
    }

    #[test]
    fn builtin_theme_families() {
        for theme in Theme::builtins() {
            let families = theme.font_families();
            match theme.id() {
                // 候选宋体（mac / Windows 各一个名字），标语 Georgia 斜体
                "sakura" => assert_eq!(families, ["Songti SC", "SimSun", "Georgia"]),
                id => assert!(families.is_empty(), "{id}"),
            }
        }
    }
}
