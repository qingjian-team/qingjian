//! 悬浮状态条（Windows）：几格并排的小条 `[中 / 英][，。/ ,.][⚙]`，每格文字居中、格间一条细线，圆角背景加阴影。
//! macOS 用菜单栏状态项，没有这一块。主题没写 `status.root` 时排法固定，尺寸与颜色取 `status` 分节；写了走节点树（`tree.rs`）。

mod cell;
mod rendered;
mod tree;

pub use cell::StatusCell;
pub use rendered::RenderedStatus;

use taffy::{AlignItems, Dimension, Display, JustifyContent, NodeId, Position, Size, Style};

use super::Renderer;
use crate::error::RenderError;
use crate::frame::Mode;
use crate::scene::{Effect, Icon, Scene, Visual};
use crate::text::TextStyle;
use crate::theme::file::ColorSpec;
use crate::theme::file::node::EffectSpec;
use crate::theme::{TextSizes, Theme};

/// 一次渲染里按倍数换算好的状态条参数。
struct Metrics<'a> {
    theme: &'a Theme,

    scale: f32,

    /// 文字样式（颜色后配）。
    font: crate::theme::FontSpec,
}

impl Metrics<'_> {
    fn px(&self, points: f32) -> f32 {
        points * self.scale
    }

    fn text_style(&self, emphasized: bool) -> TextStyle {
        let spec = &self.theme.file().status;
        let color = if emphasized {
            &spec.emphasized
        } else {
            &spec.normal
        };
        TextStyle::new(
            self.font.scaled(self.scale),
            self.font.size,
            self.theme.color(color),
            self.theme.text_gamma(),
        )
    }

    /// 状态条没有数据条件，条件颜色取 `else` 分支。
    fn effect(&self, spec: &EffectSpec) -> Effect {
        let color = match &spec.shadow().color {
            ColorSpec::Fixed(color)
            | ColorSpec::Switch {
                otherwise: color, ..
            } => color,
        };
        Effect::from_spec(spec, self.scale, self.theme.color(color))
    }
}

impl Renderer {
    /// 画状态条，返回位图与各格右边界（供点击命中）。`mode` 给节点树画法的主题显示中 / 英等状态。
    /// 设置里改了候选字号时整条按同一比例放大缩小。
    /// 固定排法：每格宽 = 内容宽 + 两侧内边距，高 = 行高 + 内边距。
    pub fn render_status(
        &mut self,
        cells: &[StatusCell],
        mode: &Mode,
        theme: &Theme,
        scale: f32,
    ) -> Result<RenderedStatus, RenderError> {
        // 整条按候选字的比例缩放：去掉设置里的字号再整体乘倍数，图标、边距、点击边界跟文字一起变，与用哪个文字样式无关
        let scale = scale * theme.candidate_scale();
        let theme = &theme.with_text_sizes(TextSizes::default());
        self.text.use_families(theme.families());
        let spec = &theme.file().status;
        if let Some(root) = &spec.root {
            return self.render_status_tree(root, cells, mode, theme, scale);
        }
        let m = Metrics {
            theme,
            scale,
            font: theme.font_ref(&spec.font),
        };
        let padding = m.px(spec.padding);
        let radius = m.px(spec.radius);
        let mut widths: Vec<f32> = cells
            .iter()
            .map(|cell| self.status_cell_width(cell, &m) + padding * 2.0)
            .collect();
        let total: f32 = widths.iter().sum();
        let content_width = total.ceil();
        // 取整多出来的零头给最后一格，让最后一格的右边界正好是内容宽
        if let Some(last) = widths.last_mut() {
            *last += content_width - total;
        }
        let content_height = (m.px(m.font.line_height) + padding).ceil();

        let mut scene = Scene::new();
        let mut children = Vec::with_capacity(cells.len());
        for (i, (cell, width)) in cells.iter().zip(&widths).enumerate() {
            let size = (*width, content_height);
            children.push(status_cell(&mut scene, cell, &m, i > 0, size)?);
        }
        let root = scene.node(
            Style {
                display: Display::Flex,
                ..Style::default()
            },
            Visual::solid(theme.color(&spec.background), radius),
            &children,
        )?;
        let effects = spec.effects.iter().map(|effect| m.effect(effect)).collect();
        scene.set_effects(root, effects);
        scene.layout(root, &mut self.text)?;

        let edges = widths
            .iter()
            .scan(0.0, |x, width| {
                *x += width;
                Some(*x)
            })
            .collect();
        Ok(RenderedStatus {
            rendered: self.rasterize(&scene, root, (content_width, content_height), scale)?,
            cell_edges: edges,
        })
    }

    /// 一格内容的宽度（像素，不含内边距）。
    fn status_cell_width(&mut self, cell: &StatusCell, m: &Metrics) -> f32 {
        match cell {
            StatusCell::Text { text, emphasized } => {
                self.text.measure(text, &m.text_style(*emphasized)).width
            }
            StatusCell::Gear => m.px(m.theme.file().status.gear_size),
        }
    }
}

/// 一格：内容在格里居中；不是第一格时左边画一条上下各缩进半个内边距的细线。
fn status_cell(
    scene: &mut Scene,
    cell: &StatusCell,
    m: &Metrics,
    separator: bool,
    (width, height): (f32, f32),
) -> Result<NodeId, RenderError> {
    let spec = &m.theme.file().status;
    let mut children = Vec::with_capacity(2);
    if separator {
        let inset = m.px(spec.padding) / 2.0;
        let style = Style {
            position: Position::Absolute,
            inset: taffy::Rect {
                left: taffy::LengthPercentageAuto::length(0.0),
                right: taffy::LengthPercentageAuto::auto(),
                top: taffy::LengthPercentageAuto::length(inset),
                bottom: taffy::LengthPercentageAuto::length(inset),
            },
            size: Size {
                width: Dimension::length(m.px(spec.separator_width)),
                height: Dimension::auto(),
            },
            ..Style::default()
        };
        let visual = Visual::solid(m.theme.color(&spec.separator), 0.0);
        children.push(scene.node(style, visual, &[])?);
    }
    let (content_style, content) = match cell {
        StatusCell::Text { text, emphasized } => (
            Style::default(),
            Visual::Text {
                text: text.clone(),
                style: m.text_style(*emphasized),
            },
        ),
        StatusCell::Gear => {
            let size = m.px(spec.gear_size);
            let style = Style {
                size: Size {
                    width: Dimension::length(size),
                    height: Dimension::length(size),
                },
                ..Style::default()
            };
            let visual = Visual::Icon {
                icon: Icon::Gear,
                size,
                color: m.theme.color(&spec.gear),
            };
            (style, visual)
        }
    };
    children.push(scene.node(content_style, content, &[])?);
    let style = Style {
        display: Display::Flex,
        justify_content: Some(JustifyContent::CENTER),
        align_items: Some(AlignItems::CENTER),
        size: Size {
            width: Dimension::length(width),
            height: Dimension::length(height),
        },
        ..Style::default()
    };
    scene.node(style, Visual::Group, &children)
}

#[cfg(test)]
mod tests {
    use super::StatusCell;
    use crate::fonts::FontLibrary;
    use crate::frame::Mode;
    use crate::renderer::Renderer;
    use crate::theme::Theme;

    #[test]
    fn cells_have_increasing_edges_ending_at_content_width() {
        // 没有系统字体的环境（CI 容器）跳过
        let Ok(library) = FontLibrary::system("zh-CN") else {
            return;
        };
        let mut renderer = Renderer::new(library);
        let cells = [
            StatusCell::text("中 · 小鹤", true),
            StatusCell::text(",.", false),
            StatusCell::Gear,
        ];
        let out = renderer
            .render_status(&cells, &Mode::default(), &Theme::light(), 2.0)
            .unwrap();
        assert_eq!(out.cell_edges.len(), 3);
        assert!(out.cell_edges.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(
            out.cell_edges.last().map(|edge| edge.round() as u32),
            Some(out.rendered.content_width)
        );
        assert!(out.rendered.pixmap.width() > out.rendered.content_width);
        assert!(out.rendered.content_x > 0);
    }
}
