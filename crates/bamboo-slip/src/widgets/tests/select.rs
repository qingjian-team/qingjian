use taffy::{AlignItems, FlexDirection, Style};

use super::super::*;
use super::Message;
use super::layout;
use crate::fonts::FontLibrary;
use crate::scene::Visual;
use crate::text::TextPainter;
use crate::ui::{Interaction, Ui};

#[test]
fn a_long_select_value_stays_inside_its_box() {
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    let theme = WidgetTheme::new(WidgetColors::LIGHT, 1.0);
    let interaction = Interaction::<Message>::new();
    let mut ui = Ui::new(&interaction);
    let long = "一个名字特别特别特别长的方案".repeat(3);
    let select = Select::new(&long)
        .width(160.0)
        .max_width(240.0)
        .build(&mut ui, &theme, "select", Message::Toggle)
        .unwrap();
    let short = Select::new("关")
        .width(160.0)
        .max_width(240.0)
        .build(&mut ui, &theme, "short", Message::Toggle)
        .unwrap();
    let root = ui
        .scene()
        .node(
            Style {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FLEX_START,
                ..Style::default()
            },
            Visual::Group,
            &[select, short],
        )
        .unwrap();
    let (mut scene, _) = ui.finish();
    layout(&mut scene, root, &mut text);
    let (_, _, width, _) = scene.rect_in(select, root).unwrap();
    assert!((width - 240.0).abs() < 0.5, "{width}");
    let (_, _, width, _) = scene.rect_in(short, root).unwrap();
    assert!((width - 160.0).abs() < 0.5, "{width}");
}
