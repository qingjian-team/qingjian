use taffy::Style;

use super::super::*;
use super::Message;
use super::{click, layout};
use crate::fonts::FontLibrary;
use crate::scene::Visual;
use crate::text::TextPainter;
use crate::ui::{Interaction, Key, Ui};

#[test]
fn menu_ignores_clicks_on_disabled_items() {
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    let mut interaction = Interaction::new();
    let theme = WidgetTheme::new(WidgetColors::LIGHT, 1.0);
    let mut ui = Ui::new(&interaction);
    let menu = Menu::new(
        &["one", "two", "three"],
        Some(0),
        (0.0, 0.0, 100.0, 20.0),
        (400.0, 400.0),
    )
    .disabled(&[1])
    .build(&mut ui, &theme, "menu", Message::Pick, Message::Up)
    .unwrap();
    let root = ui
        .scene()
        .node(
            Style {
                size: taffy::Size {
                    width: taffy::Dimension::length(400.0),
                    height: taffy::Dimension::length(400.0),
                },
                ..Style::default()
            },
            Visual::Group,
            &[menu],
        )
        .unwrap();
    let (mut scene, elements) = ui.finish();
    layout(&mut scene, root, &mut text);
    interaction.attach(&scene, root, elements).unwrap();
    let item = |index: usize| Key::ROOT.child("menu").child(index);
    assert!(click(&mut interaction, item(1)).is_empty());
    assert_eq!(click(&mut interaction, item(2)), vec![Message::Pick(2)]);
}

#[test]
fn selected_index_beyond_items_builds_without_panic() {
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    let mut interaction = Interaction::new();
    let theme = WidgetTheme::new(WidgetColors::LIGHT, 1.0);
    let mut ui = Ui::new(&interaction);
    let menu = Menu::new(
        &["one", "two"],
        Some(99),
        (0.0, 0.0, 100.0, 20.0),
        (400.0, 400.0),
    )
    .build(&mut ui, &theme, "menu", Message::Pick, Message::Up)
    .unwrap();
    let root = ui
        .scene()
        .node(
            Style {
                size: taffy::Size {
                    width: taffy::Dimension::length(400.0),
                    height: taffy::Dimension::length(400.0),
                },
                ..Style::default()
            },
            Visual::Group,
            &[menu],
        )
        .unwrap();
    let (mut scene, elements) = ui.finish();
    layout(&mut scene, root, &mut text);
    interaction.attach(&scene, root, elements).unwrap();
    // 越界选中不影响其余项可点
    let second = Key::ROOT.child("menu").child(1_usize);
    assert_eq!(click(&mut interaction, second), vec![Message::Pick(1)]);
}

#[test]
fn empty_menu_builds() {
    let interaction = Interaction::<Message>::new();
    let theme = WidgetTheme::new(WidgetColors::LIGHT, 1.0);
    let mut ui = Ui::new(&interaction);
    Menu::new(&[], None, (0.0, 0.0, 100.0, 20.0), (400.0, 400.0))
        .build(&mut ui, &theme, "menu", Message::Pick, Message::Up)
        .unwrap();
}
