//! 一行小字的几段「名目 值」（作者、大小、许可这类附带信息）：名目次要色、值正文色，段与段之间留空，放不下折行。
//! 放进 [`super::SettingRow::caption`]，比说明小一号。

mod value;

pub use value::FieldValue;

use taffy::{FlexWrap, LengthPercentageAuto, NodeId, Rect, Style};

use super::WidgetTheme;
use super::parts::row;
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::Ui;

/// 段与段之间（点）。
const FIELD_GAP: f32 = 16.0;

/// 小字的字号（点）：比说明小一号，链接这类放进来的值也用它。
pub fn field_points(theme: &WidgetTheme) -> f32 {
    theme.small - 1.0
}

pub fn fields<M>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    items: &[(&str, FieldValue)],
) -> Result<NodeId, RenderError> {
    let points = field_points(theme);
    let text = |ui: &mut Ui<M>, value: &str, color| {
        ui.scene().node(
            Style::default(),
            Visual::Text {
                text: value.to_owned(),
                style: theme.text(points, 400, color),
            },
            &[],
        )
    };
    let mut nodes = Vec::new();
    for (name, value) in items {
        let name = text(ui, name, theme.colors.secondary)?;
        let value = match value {
            FieldValue::Text(value) => text(ui, value, theme.colors.text)?,
            FieldValue::Node(node) => *node,
        };
        nodes.push(
            ui.scene()
                .node(row(theme, 4.0), Visual::Group, &[name, value])?,
        );
    }
    ui.scene().node(
        Style {
            flex_wrap: FlexWrap::Wrap,
            margin: Rect {
                left: LengthPercentageAuto::length(0.0),
                right: LengthPercentageAuto::length(0.0),
                top: LengthPercentageAuto::length(theme.px(2.0)),
                bottom: LengthPercentageAuto::length(0.0),
            },
            ..row(theme, FIELD_GAP)
        },
        Visual::Group,
        &nodes,
    )
}
