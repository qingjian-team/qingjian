//! 按钮动作的一排按钮，与做完后挂在行下面的那句话。

use bamboo_slip::taffy::{FlexDirection, NodeId, Style};
use bamboo_slip::{Button, RenderError, SettingRow, Ui, Visual, paragraph};

use crate::message::Message;
use crate::system::action::Action;
use crate::view::Page;

/// 一排按钮，各触发一个动作。
pub fn actions(
    ui: &mut Ui<Message>,
    page: &Page,
    buttons: &[(&str, Action)],
) -> Result<NodeId, RenderError> {
    let mut nodes = Vec::new();
    for &(label, action) in buttons {
        nodes.push(Button::new(label).build(
            ui,
            &page.theme,
            ("action", label),
            Message::Run(action),
        )?);
    }
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            gap: page.gap(8.0),
            ..Style::default()
        },
        Visual::Group,
        &nodes,
    )
}

/// 这一行刚做完事时，在控件下面挂上那句话。
pub fn with_notice(
    ui: &mut Ui<Message>,
    page: &Page,
    row: SettingRow,
    slot: &str,
) -> Result<NodeId, RenderError> {
    let mut row = row;
    if let Some((owner, text)) = &page.settings.notice
        && *owner == slot
    {
        row = row.below(paragraph(ui, &page.theme, text, None)?);
    }
    row.build(ui, &page.theme)
}
