//! 面板的图标：Windows 上按钮用系统图标字体，别的平台没有这套字体，用自己画的；搜索框要 [`Icon`] 就自己画放大镜。

mod menu_glyph;
mod search_glyph;

use std::sync::Arc;

use bamboo_slip::Icon;

use menu_glyph::MenuGlyph;

pub(super) use search_glyph::SearchGlyph;

/// 卡片底栏按钮的图标。
#[derive(Debug, Clone, Copy)]
pub(super) enum ActionGlyph {
    Paste,
    Copy,
}

impl ActionGlyph {
    /// Segoe Fluent Icons 与 Segoe MDL2 Assets 共用的码位。
    #[cfg(windows)]
    pub(super) fn text(self) -> &'static str {
        match self {
            Self::Paste => "\u{E77F}",
            Self::Copy => "\u{E8C8}",
        }
    }

    /// 右键菜单里同一件事的图标。
    #[cfg(not(windows))]
    pub(super) fn icon(self) -> Arc<dyn Icon> {
        Arc::new(match self {
            Self::Paste => MenuGlyph::Paste,
            Self::Copy => MenuGlyph::Copy,
        })
    }
}

/// 与 `menu_items` 同序。
pub(super) fn menu_icons() -> [Arc<dyn Icon>; 4] {
    [
        Arc::new(MenuGlyph::Paste),
        Arc::new(MenuGlyph::Copy),
        Arc::new(MenuGlyph::Edit),
        Arc::new(MenuGlyph::Delete),
    ]
}
