use super::Message;
use super::{click, frame, press};
use crate::fonts::FontLibrary;
use crate::text::TextPainter;
use crate::ui::{Interaction, Key, KeyCode};

#[test]
fn stepper_at_limit_ignores_increase() {
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    let mut interaction = Interaction::new();
    frame(&mut interaction, &mut text, 0);
    let size = Key::ROOT.child("size");
    assert!(click(&mut interaction, size.child("increase")).is_empty());
    assert_eq!(
        click(&mut interaction, size.child("decrease")),
        vec![Message::Down]
    );
    assert!(press(&mut interaction, KeyCode::ArrowUp).is_empty());
    assert_eq!(
        press(&mut interaction, KeyCode::ArrowDown),
        vec![Message::Down]
    );
}
