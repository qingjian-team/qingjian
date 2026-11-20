use super::Message;
use super::{click, frame, press};
use crate::fonts::FontLibrary;
use crate::text::TextPainter;
use crate::ui::{Interaction, Key, KeyCode};

#[test]
fn segmented_takes_focus_once_and_moves_with_arrows() {
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    let mut interaction = Interaction::new();
    frame(&mut interaction, &mut text, 0);
    press(&mut interaction, KeyCode::Tab);
    assert!(interaction.state(Key::ROOT.child("choice")).focus_ring);
    assert!(press(&mut interaction, KeyCode::ArrowLeft).is_empty());
    assert_eq!(
        press(&mut interaction, KeyCode::ArrowRight),
        vec![Message::Pick(1)]
    );
    // 单个选项不单独拿焦点：下一个 Tab 直接到开关
    press(&mut interaction, KeyCode::Tab);
    assert!(interaction.state(Key::ROOT.child("switch")).focused);
    assert_eq!(
        press(&mut interaction, KeyCode::Space),
        vec![Message::Toggle]
    );
}

#[test]
fn clicking_an_option_picks_it() {
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    let mut interaction = Interaction::new();
    frame(&mut interaction, &mut text, 0);
    let third = Key::ROOT.child("choice").child(2_usize);
    assert_eq!(click(&mut interaction, third), vec![Message::Pick(2)]);
    assert!(interaction.state(Key::ROOT.child("choice")).focused);
}
