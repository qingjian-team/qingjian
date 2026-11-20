//! 控件层测试：按控件分文件，共用的搭帧、按键、点击 helper 留在本文件。

use taffy::{FlexDirection, NodeId, Style};

use super::*;
use crate::scene::{Scene, Visual};
use crate::text::TextPainter;
use crate::ui::{Event, Interaction, Key, KeyCode, KeyPress, Modifiers, Ui};

mod button;
mod layout_fuzz;
mod menu;
mod paragraph;
mod scroll_area;
mod segmented;
mod select;
mod setting_row;
mod slider;
mod stepper;
mod text_field;
mod text_painter;
mod transition;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Message {
    Pick(usize),
    Toggle,
    Down,
    Up,
}

/// 一排：分段选择（选中第 `selected` 个）、开关、到了上限的步进框。
pub(super) fn frame(
    interaction: &mut Interaction<Message>,
    text: &mut TextPainter,
    selected: usize,
) {
    let theme = WidgetTheme::new(WidgetColors::LIGHT, 1.0);
    let mut ui = Ui::new(interaction);
    let segmented = Segmented::new(&["一", "二", "三"], selected)
        .build(&mut ui, &theme, "choice", Message::Pick)
        .unwrap();
    let switch = Switch::new(true)
        .build(&mut ui, &theme, "switch", Message::Toggle)
        .unwrap();
    let stepper = Stepper::new("40 pt")
        .limits(true, false)
        .build(&mut ui, &theme, "size", Message::Down, Message::Up)
        .unwrap();
    let root = ui
        .scene()
        .node(
            Style {
                flex_direction: FlexDirection::Row,
                ..Style::default()
            },
            Visual::Group,
            &[segmented, switch, stepper],
        )
        .unwrap();
    let (mut scene, elements) = ui.finish();
    layout(&mut scene, root, text);
    interaction.attach(&scene, root, elements).unwrap();
}

pub(super) fn layout(scene: &mut Scene, root: NodeId, text: &mut TextPainter) {
    scene.layout(root, text).unwrap();
}

pub(super) fn press(interaction: &mut Interaction<Message>, key: KeyCode) -> Vec<Message> {
    interaction
        .handle(&Event::Key(KeyPress::new(key, Modifiers::default())))
        .messages
}

pub(super) fn click(interaction: &mut Interaction<Message>, key: Key) -> Vec<Message> {
    let (x, y, width, height) = interaction.rect(key).unwrap();
    interaction.handle(&Event::PointerMoved {
        x: x + width / 2.0,
        y: y + height / 2.0,
    });
    interaction.handle(&Event::PointerDown);
    interaction.handle(&Event::PointerUp).messages
}
