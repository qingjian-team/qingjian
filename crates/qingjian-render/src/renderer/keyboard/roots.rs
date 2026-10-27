//! 五笔键：字根排成四列网格铺满键面，底下一行左边键名、右边区位号，与纸面字根图同一个排法。
//! 字根是矢量轮廓（见 [`Root`]），不经过文字排版，各平台画出来一样。

use bamboo_slip::taffy::{AlignItems, Dimension, JustifyContent, NodeId, Size, Style};
use bamboo_slip::{RenderError, Scene, Visual};

use super::{INSET, Metrics, absolute, paint, text};
use crate::keyboard::{Keycap, Root};

/// 每行几个字根。
const COLUMNS: usize = 4;

/// 字在格子里占多大（其余留作字距）。
const FILL: f32 = 0.88;

pub(super) fn key(
    scene: &mut Scene,
    cap: &Keycap,
    m: &Metrics,
    (left, top): (f32, f32),
    (width, height): (f32, f32),
) -> Result<NodeId, RenderError> {
    let spec = &m.theme.file().keyboard;
    let paint = paint(&cap.state, m);
    let inset = m.px(INSET);
    let footer = m.px(m.label_font.line_height);
    let rows = cap.roots.len().div_ceil(COLUMNS).max(1);
    let cell =
        ((width - inset * 2.0) / COLUMNS as f32).min((height - inset * 2.0 - footer) / rows as f32);
    let size = (m.root_font.size.min(cell / m.scale * FILL)) * m.scale;

    let mut children = Vec::with_capacity(cap.roots.len() + 2);
    for (i, root) in cap.roots.iter().enumerate() {
        let (row, column) = (i / COLUMNS, i % COLUMNS);
        let glyph = glyph(scene, root, size, paint.lines)?;
        let cell_style = Style {
            display: bamboo_slip::taffy::Display::Flex,
            justify_content: JustifyContent::CENTER,
            align_items: AlignItems::CENTER,
            ..absolute(
                (Some(inset + column as f32 * cell), None),
                Some(inset + row as f32 * cell),
                Some((cell, cell)),
            )
        };
        children.push(scene.node(cell_style, Visual::Group, &[glyph])?);
    }
    let bottom = height - inset - footer;
    children.push(text(
        scene,
        &cap.label,
        m.style(m.label_font, paint.key),
        absolute((Some(inset), None), Some(bottom), None),
    )?);
    if let Some(corner) = &cap.corner {
        children.push(text(
            scene,
            corner,
            m.style(m.label_font, paint.corner),
            absolute((None, Some(inset)), Some(bottom), None),
        )?);
    }
    let style = absolute((Some(left), None), Some(top), Some((width, height)));
    scene.node(
        style,
        Visual::solid(paint.fill, m.px(spec.key_radius)),
        &children,
    )
}

/// 一个字根：边长 `size` 的矢量图标。
fn glyph(
    scene: &mut Scene,
    root: &Root,
    size: f32,
    color: bamboo_slip::Color,
) -> Result<NodeId, RenderError> {
    scene.node(
        Style {
            size: Size {
                width: Dimension::length(size),
                height: Dimension::length(size),
            },
            flex_shrink: 0.0,
            ..Style::default()
        },
        Visual::Icon {
            icon: root.icon(),
            size,
            color,
        },
        &[],
    )
}
