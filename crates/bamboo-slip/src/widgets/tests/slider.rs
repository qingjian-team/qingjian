use taffy::{FlexDirection, Style};

use super::super::*;
use super::layout;
use crate::fonts::FontLibrary;
use crate::scene::Visual;
use crate::text::TextPainter;
use crate::ui::{Event, Interaction, Key, KeyCode, KeyPress, Modifiers, Ui};

#[derive(Debug, Clone, PartialEq)]
enum SliderMsg {
    Change(f32),
}

fn frame(
    interaction: &mut Interaction<SliderMsg>,
    text: &mut TextPainter,
    value: f32,
    min: f32,
    max: f32,
    disabled: bool,
) {
    let theme = WidgetTheme::new(WidgetColors::LIGHT, 1.0);
    let mut ui = Ui::new(interaction);
    let slider = Slider::new(value, min, max)
        .disabled(disabled)
        .build(&mut ui, &theme, "slider", SliderMsg::Change)
        .unwrap();
    let root = ui
        .scene()
        .node(
            Style {
                flex_direction: FlexDirection::Row,
                ..Style::default()
            },
            Visual::Group,
            &[slider],
        )
        .unwrap();
    let (mut scene, elements) = ui.finish();
    layout(&mut scene, root, text);
    interaction.attach(&scene, root, elements).unwrap();
}

fn press(interaction: &mut Interaction<SliderMsg>, key: KeyCode) -> Vec<SliderMsg> {
    interaction
        .handle(&Event::Key(KeyPress::new(key, Modifiers::default())))
        .messages
}

fn focus(interaction: &mut Interaction<SliderMsg>) {
    let (x, y, width, height) = interaction.rect(Key::ROOT.child("slider")).unwrap();
    interaction.handle(&Event::PointerMoved {
        x: x + width / 2.0,
        y: y + height / 2.0,
    });
    interaction.handle(&Event::PointerDown);
    interaction.handle(&Event::PointerUp);
}

#[test]
fn home_end_and_step_report_clamped_values() {
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    let mut interaction = Interaction::new();
    frame(&mut interaction, &mut text, 5.0, 0.0, 10.0, false);
    focus(&mut interaction);
    assert_eq!(
        press(&mut interaction, KeyCode::Home),
        vec![SliderMsg::Change(0.0)]
    );
    assert_eq!(
        press(&mut interaction, KeyCode::End),
        vec![SliderMsg::Change(10.0)]
    );
    // 当前值 5，右移一步取整到 6
    assert_eq!(
        press(&mut interaction, KeyCode::ArrowRight),
        vec![SliderMsg::Change(6.0)]
    );
}

#[test]
fn out_of_range_value_clamps_to_bounds() {
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    let mut interaction = Interaction::new();
    frame(&mut interaction, &mut text, -5.0, 0.0, 10.0, false);
    focus(&mut interaction);
    assert_eq!(
        press(&mut interaction, KeyCode::Home),
        vec![SliderMsg::Change(0.0)]
    );

    let mut interaction = Interaction::new();
    frame(&mut interaction, &mut text, 99.0, 0.0, 10.0, false);
    focus(&mut interaction);
    assert_eq!(
        press(&mut interaction, KeyCode::End),
        vec![SliderMsg::Change(10.0)]
    );
}

#[test]
fn disabled_slider_ignores_keys() {
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    let mut interaction = Interaction::new();
    frame(&mut interaction, &mut text, 5.0, 0.0, 10.0, true);
    focus(&mut interaction);
    assert!(press(&mut interaction, KeyCode::Home).is_empty());
    assert!(press(&mut interaction, KeyCode::End).is_empty());
    assert!(press(&mut interaction, KeyCode::ArrowRight).is_empty());
}

#[test]
fn value_at_boundary_emits_nothing_on_same_side() {
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    let mut interaction = Interaction::new();
    frame(&mut interaction, &mut text, 0.0, 0.0, 10.0, false);
    focus(&mut interaction);
    assert!(press(&mut interaction, KeyCode::ArrowLeft).is_empty());
    assert!(press(&mut interaction, KeyCode::Home).is_empty());

    let mut interaction = Interaction::new();
    frame(&mut interaction, &mut text, 10.0, 0.0, 10.0, false);
    focus(&mut interaction);
    assert!(press(&mut interaction, KeyCode::ArrowRight).is_empty());
    assert!(press(&mut interaction, KeyCode::End).is_empty());
}
