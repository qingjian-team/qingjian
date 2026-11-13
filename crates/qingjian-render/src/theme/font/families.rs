//! 主题的字体表：缺省字族与各命名样式的回退链进引擎的字族表（[`FamilyTable`]），命名样式里只存序号。
//! 加载主题时建一次；画笔按序号查链、挑第一个装了的字族。

use std::collections::HashMap;
use std::sync::Arc;

use bamboo_slip::{FamilyList, FamilyTable, FontSpec};

use crate::theme::file::{FontStyle, TextSettings};

/// 设置里选了字体时改用它的样式：候选、注释、序号、拼音这些显示输入内容的字。
/// 标语这类装饰字（樱花的 Georgia 斜体）仍按主题写的字族。
const USER_FONT_STYLES: [&str; 4] = ["candidate", "annotation", "index", "preedit"];

#[derive(Debug, Default)]
pub(crate) struct FontFamilies {
    /// 回退链，交给画笔；同一主题的浅色 / 深色共用这一份。
    table: Arc<FamilyTable>,

    /// 命名样式（点），字族已换成序号。
    styles: HashMap<String, FontSpec>,
}

impl FontFamilies {
    /// `user_font`：设置里选了字体（壳已把它登记成 `system`），主题的缺省字族与 [`USER_FONT_STYLES`] 都换成它。
    pub(crate) fn new(text: &TextSettings, user_font: bool) -> Self {
        let default = match &text.family {
            Some(family) if !user_font => family.clone(),
            _ => FamilyList::system(),
        };
        let mut table = FamilyTable::new(default);
        // 按样式名排，字族表的顺序（也是壳查字体的顺序）每次加载都一样
        let mut named: Vec<_> = text.styles.iter().collect();
        named.sort_by(|a, b| a.0.cmp(b.0));
        let styles = named
            .into_iter()
            .map(|(name, spec)| {
                let font = FontSpec {
                    size: spec.size,
                    line_height: spec.line_height,
                    weight: spec.weight,
                    italic: spec.style == FontStyle::Italic,
                    family: match &spec.family {
                        Some(list) if !(user_font && USER_FONT_STYLES.contains(&name.as_str())) => {
                            table.intern(list)
                        }
                        _ => bamboo_slip::FamilyId(0),
                    },
                };
                (name.clone(), font)
            })
            .collect();
        Self {
            table: Arc::new(table),
            styles,
        }
    }

    pub(crate) fn style(&self, name: &str) -> Option<FontSpec> {
        self.styles.get(name).copied()
    }

    pub(crate) fn table(&self) -> &Arc<FamilyTable> {
        &self.table
    }
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

    /// #957：设置里选了字体，写死字族的主题也换成它；装饰字照旧。
    #[test]
    fn user_font_replaces_theme_families_for_text_styles() {
        let system = |theme: &Theme, name: &str| {
            let id = theme.font(name).family.0;
            theme.families().chains()[id as usize].0.clone()
        };
        let sakura = Theme::builtin("sakura", false).unwrap();
        assert_eq!(system(&sakura, "candidate")[0], "Songti SC");
        let sakura = sakura.with_user_font(true);
        for name in ["candidate", "annotation", "index", "preedit"] {
            assert_eq!(system(&sakura, name), ["system"], "{name}");
        }
        assert_eq!(system(&sakura, "tagline")[0], "Georgia");

        let dafeiyu = Theme::builtin("dafeiyu", false)
            .unwrap()
            .with_user_font(true);
        assert_eq!(system(&dafeiyu, "candidate"), ["system"]);
        assert_eq!(system(&dafeiyu, "status"), ["system"]);
        // 关掉（选回缺省字体）恢复主题的字族
        assert_eq!(
            system(&dafeiyu.with_user_font(false), "candidate")[0],
            "PingFang SC"
        );
    }

    #[test]
    fn builtin_theme_families() {
        for theme in Theme::builtins() {
            let families = theme.font_families();
            match theme.id() {
                // 候选宋体（mac / Windows 各一个名字），标语 Georgia 斜体
                "sakura" => assert_eq!(families, ["Songti SC", "SimSun", "Georgia"]),
                "dafeiyu" | "bocchi" => assert_eq!(families, ["PingFang SC", "Microsoft YaHei"]),
                // 「01」用 Avenir Next（mac）/ Segoe UI（Windows）
                "miku" => assert_eq!(
                    families,
                    ["PingFang SC", "Microsoft YaHei", "Avenir Next", "Segoe UI"]
                ),
                id => assert!(families.is_empty(), "{id}"),
            }
        }
    }
}
