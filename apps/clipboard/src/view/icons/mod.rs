//! 面板的图标：按钮用系统图标字体，搜索框要 [`Icon`] 就自己画放大镜。

mod menu_glyph;
mod search_glyph;

use std::sync::Arc;

use bamboo_slip::Icon;

use menu_glyph::MenuGlyph;

pub(super) use search_glyph::SearchGlyph;

/// Segoe Fluent Icons 与 Segoe MDL2 Assets 共用的码位。
pub(super) const PASTE: &str = "\u{E77F}";

pub(super) const COPY: &str = "\u{E8C8}";

/// 与 `menu_items` 同序。
pub(super) fn menu_icons() -> [Arc<dyn Icon>; 4] {
    [
        Arc::new(MenuGlyph::Paste),
        Arc::new(MenuGlyph::Copy),
        Arc::new(MenuGlyph::Edit),
        Arc::new(MenuGlyph::Delete),
    ]
}
