//! 双拼键：左上键名，右上角标（翘舌声母），下方几行小字（韵母，或敲了声母后能拼成的音节）。

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{RenderError, Scene, Visual};

use super::{INSET, Metrics, absolute, paint, text};
use crate::keyboard::{Keycap, KeycapState};

/// 一个键：盒子绝对定位在键盘里，键名、角标、小字再绝对定位在盒子里。
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
    let mut children = Vec::with_capacity(2 + cap.lines.len());
    children.push(text(
        scene,
        &cap.label,
        m.style(m.key_font, paint.key),
        absolute((Some(inset), None), Some(inset), None),
    )?);
    if let Some(corner) = &cap.corner {
        children.push(text(
            scene,
            corner,
            m.style(m.label_font, paint.corner),
            absolute((None, Some(inset)), Some(inset), None),
        )?);
    }
    let next;
    let lines: &[String] = match &cap.state {
        KeycapState::Next(syllable) => {
            next = [syllable.clone()];
            &next
        }
        _ => &cap.lines,
    };
    let line_height = m.px(m.label_font.line_height);
    for (i, line) in lines.iter().enumerate() {
        let top = height - inset - (lines.len() - i) as f32 * line_height;
        children.push(text(
            scene,
            line,
            m.style(m.label_font, paint.lines),
            absolute((Some(inset), None), Some(top), None),
        )?);
    }
    let style = absolute((Some(left), None), Some(top), Some((width, height)));
    scene.node(
        style,
        Visual::solid(paint.fill, m.px(spec.key_radius)),
        &children,
    )
}
