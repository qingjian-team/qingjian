use taffy::{FlexDirection, Style};

use super::super::*;
use super::Message;
use super::layout;
use crate::fonts::FontLibrary;
use crate::scene::Visual;
use crate::text::TextPainter;
use crate::ui::{Interaction, Ui};

#[test]
fn paragraph_wraps_to_the_column_width() {
    let interaction = Interaction::<Message>::new();
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    let theme = WidgetTheme::new(WidgetColors::LIGHT, 1.0);
    let mut ui = Ui::new(&interaction);
    let words = "word ".repeat(40);
    let note = paragraph(&mut ui, &theme, &words, None).unwrap();
    let root = ui
        .scene()
        .node(
            Style {
                size: taffy::Size {
                    width: taffy::Dimension::length(120.0),
                    height: taffy::Dimension::auto(),
                },
                flex_direction: FlexDirection::Column,
                ..Style::default()
            },
            Visual::Group,
            &[note],
        )
        .unwrap();
    let (mut scene, _) = ui.finish();
    layout(&mut scene, root, &mut text);
    let (_, _, width, height) = scene.rect_in(note, root).unwrap();
    let line = text.measure(
        "word",
        &theme.text(theme.small, 400, theme.colors.secondary),
    );
    assert!(width <= 120.0);
    assert!(height > line.height * 3.0, "{height}");
}
