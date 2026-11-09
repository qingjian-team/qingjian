//! 候选右键菜单：bamboo-slip 的 [`PopupMenu`] 画成位图，各项的矩形就是点击区域（[`HitTarget::MenuItem`]）。
//! 配色用控件的那一套（与设置页的下拉菜单一样），只分深浅，不随候选窗主题变。菜单窗口要单独一个 [`Renderer`]，
//! 免得和候选窗的上一帧配对做过渡。

use std::time::Instant;

use bamboo_slip::{PopupMenu, RenderError, Scene, WidgetColors, WidgetTheme};

use super::{HitRegion, HitTarget, Rendered, Renderer};

impl Renderer {
    /// 画一份菜单：`hovered` 是鼠标停在哪一项上，`scale` 是点 → 像素的倍数。
    pub fn render_menu(
        &mut self,
        items: &[&str],
        hovered: Option<usize>,
        dark: bool,
        scale: f32,
    ) -> Result<Rendered, RenderError> {
        let colors = if dark {
            WidgetColors::DARK
        } else {
            WidgetColors::LIGHT
        };
        let theme = WidgetTheme::new(colors, scale);
        let mut scene = Scene::new();
        let (root, rows) = PopupMenu::new(items)
            .hovered(hovered)
            .build(&mut scene, &theme)?;
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
