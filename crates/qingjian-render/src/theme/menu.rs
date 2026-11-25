//! 候选右键菜单的配色：主题的 `menu` 一节怎么变成 [`crate::MenuStyle`]。
//!
//! 单独一个文件，`mod.rs` 的行数挨着上限，菜单这一块又是自成一体的一小摊。

use bamboo_slip::{Color, WidgetColors};

use super::Theme;
use super::file::ColorRef;

impl Theme {
    /// 候选右键菜单按这个主题画成什么样（配色 + 圆角），壳直接传给 [`crate::Renderer::render_menu`]。
    ///
    /// 主题没写 `menu` 一节时就是控件自己的浅色 / 深色，与设置页的下拉菜单一样。
    pub fn menu_style(&self, dark: bool) -> crate::MenuStyle {
        let fallback = if dark {
            WidgetColors::DARK
        } else {
            WidgetColors::LIGHT
        };
        crate::MenuStyle {
            colors: self.menu_colors(fallback),
            radius: self.file.menu.radius,
            border_width: self.file.menu.border_width,
        }
    }

    /// 候选右键菜单的配色：主题写了 `menu` 就用主题的，没写的项退回控件的浅色 / 深色默认。
    ///
    /// 只覆盖菜单真正用到的几个色（底、行、悬停、字、线），其余沿用 `fallback`——
    /// 弹出菜单控件还会用到强调色与槽色，那些在候选菜单里不出现，跟着默认走就行。
    pub(crate) fn menu_colors(&self, fallback: WidgetColors) -> WidgetColors {
        let spec = &self.file.menu;
        let pick = |value: &Option<ColorRef>, default: Color| {
            value.as_ref().map_or(default, |color| self.color(color))
        };
        WidgetColors {
            text: pick(&spec.text, fallback.text),
            popup: pick(&spec.background, fallback.popup),
            surface: pick(&spec.background, fallback.surface),
            hover: pick(&spec.hover, fallback.hover),
            selected: pick(&spec.hover, fallback.selected),
            line: pick(&spec.line, fallback.line),
            ..fallback
        }
    }
}
