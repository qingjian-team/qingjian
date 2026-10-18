//! 段落：按父节点给的宽度折行的小字，用在设置页里少数非写不可的说明（隐私、固定键）。

use taffy::{NodeId, Style};

use super::WidgetTheme;
use crate::color::Color;
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::Ui;

/// `color` 为 `None` 时用次要色。宽度跟着父节点走（放在竖排容器里会拉满）。
pub fn paragraph<M>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    text: &str,
    color: Option<Color>,
) -> Result<NodeId, RenderError> {
    let style = theme
        .text(theme.small, 400, color.unwrap_or(theme.colors.secondary))
        .wrapped();
    // 小字的行高按 1.6 倍放松，折成几行时好读
    let style = crate::text::TextStyle {
        line_height: (style.size * 1.6).round(),
        ..style
    };
    ui.scene().node(
        Style {
            flex_shrink: 1.0,
            ..Style::default()
        },
        Visual::Text {
            text: text.to_owned(),
            style,
        },
        &[],
    )
}
