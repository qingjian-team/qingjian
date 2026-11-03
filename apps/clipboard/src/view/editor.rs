//! 编辑页：整个面板换成一个多行输入框。搜索框不在树里，焦点才会落到这里。

use bamboo_slip::taffy::{JustifyContent, LengthPercentageAuto, NodeId, Rect, Style};
use bamboo_slip::{Button, RenderError, TextField, Ui, Visual};

use super::{Context, PANEL_GAP, PANEL_PADDING, gap};
use crate::app::{Editing, Message};
use crate::i18n::tr;

pub(super) fn editor(
    ui: &mut Ui<Message>,
    context: &Context,
    editing: &Editing,
    size: (u32, u32),
) -> Result<Vec<NodeId>, RenderError> {
    let theme = &context.theme;
    let available =
        size.1 as f32 - theme.px(PANEL_PADDING * 2.0 + PANEL_GAP + theme.control_height + 16.0);
    let rows = ((available / theme.px(theme.font.line_height)) as usize).max(3);
    let field = TextField::new(&editing.text)
        .stretch()
        .autofocus()
        .rows(rows)
        .max_rows(rows)
        .build(
            ui,
            theme,
            "editor",
            Message::EditChange,
            None,
            Some(Message::EditCancel),
        )?;
    let cancel =
        Button::new(tr("editor.cancel")).build(ui, theme, "editor-cancel", Message::EditCancel)?;
    let save = Button::new(tr("editor.save")).primary().build(
        ui,
        theme,
        "editor-save",
        Message::EditSave,
    )?;
    let buttons = ui.scene().node(
        Style {
            flex_shrink: 0.0,
            margin: Rect {
                top: LengthPercentageAuto::auto(),
                left: LengthPercentageAuto::length(0.0),
                right: LengthPercentageAuto::length(0.0),
                bottom: LengthPercentageAuto::length(0.0),
            },
            justify_content: JustifyContent::END,
            gap: gap(theme, 8.0),
            ..Style::default()
        },
        Visual::Group,
        &[cancel, save],
    )?;
    Ok(vec![field, buttons])
}
