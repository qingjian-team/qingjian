//! 候选右键菜单：bamboo-slip 的 [`PopupMenu`] 画成位图，各项的矩形就是点击区域（[`HitTarget::MenuItem`]）。
//!
//! 配色来自主题的 `menu` 一节（见 `theme::Theme::menu_colors`）；主题没写这一节时用控件自己的
//! 浅色 / 深色默认——与设置页的下拉菜单同一套。菜单窗口要单独一个 [`Renderer`]，
//! 免得和候选窗的上一帧配对做过渡。

use std::time::Instant;

use bamboo_slip::{PopupMenu, RenderError, Scene, WidgetColors, WidgetTheme};

use super::{HitRegion, HitTarget, Rendered, Renderer};

/// 菜单画成什么样：配色与圆角。由主题里写，壳在弹菜单前解析好传进来。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuStyle {
    pub colors: WidgetColors,

    /// 圆角；`None` 用控件默认值。方角主题（墨岩）写 0。
    pub radius: Option<f32>,

    /// 卡片边框宽度；`None` 用控件默认值（1 点）。
    pub border_width: Option<f32>,
}

impl Default for MenuStyle {
    fn default() -> Self {
        Self::system(false)
    }
}

impl MenuStyle {
    /// 主题没写 `menu` 时的样子：控件自己的浅色 / 深色。
    pub fn system(dark: bool) -> Self {
        Self {
            colors: if dark {
                WidgetColors::DARK
            } else {
                WidgetColors::LIGHT
            },
            radius: None,
            border_width: None,
        }
    }
}

impl Renderer {
    /// 画一份菜单：`hovered` 是鼠标停在哪一项上，`scale` 是点 → 像素的倍数。
    pub fn render_menu(
        &mut self,
        items: &[&str],
        hovered: Option<usize>,
        style: &MenuStyle,
        scale: f32,
    ) -> Result<Rendered, RenderError> {
        let theme = WidgetTheme::new(style.colors, scale);
        let mut scene = Scene::new();
        let mut menu = PopupMenu::new(items).hovered(hovered);
        if let Some(radius) = style.radius {
            menu = menu.radius(radius);
        }
        if let Some(width) = style.border_width {
            menu = menu.border_width(width);
        }
        let (root, rows) = menu.build(&mut scene, &theme)?;
        let content = scene.layout(root, self.stage.text())?;
        let mut hits = Vec::with_capacity(rows.len());
        for (index, row) in rows.into_iter().enumerate() {
            let (x, y, width, height) = scene.rect_in(row, root)?;
            hits.push(HitRegion {
                x,
                y,
                width,
                height,
                target: HitTarget::MenuItem(index),
            });
        }
        self.hits = hits;
        let painted = self
            .stage
            .present(scene, root, content, 0, scale, Instant::now())?;
        Ok(Rendered::new(painted, self.hits.clone()))
    }
}
