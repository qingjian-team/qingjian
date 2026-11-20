use taffy::{FlexDirection, Style};

use super::super::*;
use super::layout;
use crate::fonts::FontLibrary;
use crate::scene::Visual;
use crate::text::TextPainter;
use crate::ui::{Event, Interaction, Key, Ui};

#[derive(Debug, Clone, PartialEq)]
enum ButtonMsg {
    Click,
}

fn click(interaction: &mut Interaction<ButtonMsg>, key: Key) -> Vec<ButtonMsg> {
    let (x, y, width, height) = interaction.rect(key).unwrap();
    interaction.handle(&Event::PointerMoved {
        x: x + width / 2.0,
        y: y + height / 2.0,
    });
    interaction.handle(&Event::PointerDown);
    interaction.handle(&Event::PointerUp).messages
}

#[test]
fn click_emits_message_and_disabled_is_silent() {
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    let mut interaction = Interaction::new();
    let theme = WidgetTheme::new(WidgetColors::LIGHT, 1.0);
    let mut ui = Ui::new(&interaction);
    let normal = Button::new("确定")
        .build(&mut ui, &theme, "ok", ButtonMsg::Click)
        .unwrap();
    let disabled = Button::new("取消")
        .disabled(true)
        .build(&mut ui, &theme, "no", ButtonMsg::Click)
        .unwrap();
    let root = ui
        .scene()
        .node(
            Style {
                flex_direction: FlexDirection::Row,
                ..Style::default()
            },
            Visual::Group,
            &[normal, disabled],
        )
        .unwrap();
    let (mut scene, elements) = ui.finish();
    layout(&mut scene, root, &mut text);
    interaction.attach(&scene, root, elements).unwrap();

    assert_eq!(
        click(&mut interaction, Key::ROOT.child("ok")),
        vec![ButtonMsg::Click]
    );
    assert!(click(&mut interaction, Key::ROOT.child("no")).is_empty());
}
