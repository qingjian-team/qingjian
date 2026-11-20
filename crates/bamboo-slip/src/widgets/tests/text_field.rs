use taffy::{AlignItems, FlexDirection, Style};

use super::super::*;
use super::layout;
use crate::fonts::FontLibrary;
use crate::scene::Visual;
use crate::text::TextPainter;
use crate::ui::{Event, Interaction, Key, KeyCode, KeyPress, Modifiers, TextInput, Ui};

#[derive(Debug, Clone, PartialEq)]
enum Field {
    Edit(Edit),
    Commit,
}

/// 一个输入框加一个开关；返回布好局、认回交互的一帧。
fn field_frame(interaction: &mut Interaction<Field>, text: &mut TextPainter, edit: &TextEdit) {
    let theme = WidgetTheme::new(WidgetColors::LIGHT, 1.0);
    let mut ui = Ui::new(interaction);
    let field = TextField::new(edit)
        .build(
            &mut ui,
            &theme,
            "field",
            Field::Edit,
            Some(Field::Commit),
            None,
        )
        .unwrap();
    let switch = Switch::new(false)
        .build(&mut ui, &theme, "switch", Field::Commit)
        .unwrap();
    let root = ui
        .scene()
        .node(
            Style {
                flex_direction: FlexDirection::Row,
                ..Style::default()
            },
            Visual::Group,
            &[field, switch],
        )
        .unwrap();
    let (mut scene, elements) = ui.finish();
    layout(&mut scene, root, text);
    interaction.attach(&scene, root, elements).unwrap();
}

/// 在输入框里按下的位置：(x 占元素宽度的比例, 元素)。
fn press_at(interaction: &mut Interaction<Field>, key: Key, fraction: f32) -> Vec<Field> {
    let (x, y, width, height) = interaction.rect(key).unwrap();
    interaction.handle(&Event::PointerMoved {
        x: x + width * fraction,
        y: y + height / 2.0,
    });
    let messages = interaction.handle(&Event::PointerDown).messages;
    interaction.handle(&Event::PointerUp);
    messages
}

/// 多行输入框按框宽折行，设了最多几行时不再往下长。
#[test]
fn multiline_field_wraps_to_its_width() {
    let interaction = Interaction::<Field>::new();
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    let theme = WidgetTheme::new(WidgetColors::LIGHT, 1.0);
    let mut ui = Ui::new(&interaction);
    let edit = TextEdit::multiline("word ".repeat(40));
    let grown = TextField::new(&edit)
        .width(160.0)
        .rows(1)
        .build(&mut ui, &theme, "grown", |_| Field::Commit, None, None)
        .unwrap();
    let capped = TextField::new(&edit)
        .width(160.0)
        .rows(1)
        .max_rows(3)
        .build(&mut ui, &theme, "capped", |_| Field::Commit, None, None)
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
            &[grown, capped],
        )
        .unwrap();
    let (mut scene, _) = ui.finish();
    layout(&mut scene, root, &mut text);
    let line = theme
        .text(theme.font.size - 1.0, 400, theme.colors.text)
        .line_height;
    let (_, _, width, grown) = scene.rect_in(grown, root).unwrap();
    let (_, _, _, capped) = scene.rect_in(capped, root).unwrap();
    assert!(width <= 160.5, "{width}");
    assert!(grown > line * 6.0, "{grown}");
    assert!(capped < line * 4.0, "{capped}");
}

/// 点字的左半边光标落在它前面、右半边落在后面；点在文字后面的空白落到末尾。没拿焦点时点也一样。
#[test]
fn clicking_text_moves_the_caret() {
    let mut interaction = Interaction::new();
    let mut text = TextPainter::new(FontLibrary::system("zh-CN").unwrap());
    let edit = TextEdit::new("你好 hello");
    field_frame(&mut interaction, &mut text, &edit);
    let field = Key::ROOT.child("field");
    // 「好 」是一个单位（字节 3..7）：左边一点落在 3，右边落在 7
    assert_eq!(
        press_at(&mut interaction, field.child(("at", 3_usize)), 0.1),
        vec![Field::Edit(Edit::Caret(3))]
    );
    assert_eq!(
        press_at(&mut interaction, field.child(("at", 3_usize)), 0.95),
        vec![Field::Edit(Edit::Caret(7))]
    );
    // 「hello」按字符数均分：中间偏右落在 hel|lo
    assert_eq!(
        press_at(&mut interaction, field.child(("at", 7_usize)), 0.55),
        vec![Field::Edit(Edit::Caret(10))]
    );
    assert_eq!(
        press_at(&mut interaction, field.child("view"), 0.99),
        vec![Field::Edit(Edit::Caret(12))]
    );
    let mut edit = edit;
    edit.apply(&Edit::Caret(4));
    assert_eq!((edit.caret(), edit.selection()), (3, 3..3));
}

/// 秘密输入框聚焦时关输入法，普通输入框开着。
#[test]
fn secret_field_disallows_ime() {
    let mut interaction = Interaction::new();
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    let edit = TextEdit::new("");
    let theme = WidgetTheme::new(WidgetColors::LIGHT, 1.0);
    let mut ui = Ui::new(&interaction);
    let plain = TextField::new(&edit)
        .build(&mut ui, &theme, "plain", Field::Edit, None, None)
        .unwrap();
    let secret = TextField::new(&edit)
        .secret(true)
        .build(&mut ui, &theme, "secret", Field::Edit, None, None)
        .unwrap();
    let root = ui
        .scene()
        .node(
            Style {
                flex_direction: FlexDirection::Row,
                ..Style::default()
            },
            Visual::Group,
            &[plain, secret],
        )
        .unwrap();
    let (mut scene, elements) = ui.finish();
    layout(&mut scene, root, &mut text);
    interaction.attach(&scene, root, elements).unwrap();

    let (x, y, width, height) = interaction.rect(Key::ROOT.child("plain")).unwrap();
    interaction.handle(&Event::PointerMoved {
        x: x + width / 2.0,
        y: y + height / 2.0,
    });
    interaction.handle(&Event::PointerDown);
    interaction.handle(&Event::PointerUp);
    assert!(interaction.ime_allowed(), "普通输入框开着输入法");

    let (x, y, width, height) = interaction.rect(Key::ROOT.child("secret")).unwrap();
    interaction.handle(&Event::PointerMoved {
        x: x + width / 2.0,
        y: y + height / 2.0,
    });
    interaction.handle(&Event::PointerDown);
    interaction.handle(&Event::PointerUp);
    assert!(!interaction.ime_allowed(), "秘密输入框聚焦时宿主关输入法");
}

#[test]
fn text_goes_to_the_focused_field_and_leaving_commits() {
    let mut interaction = Interaction::new();
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    let mut edit = TextEdit::new("");
    field_frame(&mut interaction, &mut text, &edit);
    let typed = Event::Text(TextInput::Insert("a".to_owned()));
    assert!(
        interaction.handle(&typed).messages.is_empty(),
        "没焦点时不收字"
    );
    assert!(interaction.text_target().is_none());

    let (x, y, width, height) = interaction.rect(Key::ROOT.child("field")).unwrap();
    interaction.handle(&Event::PointerMoved {
        x: x + width / 2.0,
        y: y + height / 2.0,
    });
    interaction.handle(&Event::PointerDown);
    interaction.handle(&Event::PointerUp);
    assert!(interaction.text_target().is_some());
    for message in interaction.handle(&typed).messages {
        let Field::Edit(change) = message else {
            panic!("打字只该产出编辑");
        };
        edit.apply(&change);
    }
    assert_eq!(edit.text(), "a");
    field_frame(&mut interaction, &mut text, &edit);

    let left = interaction.handle(&Event::Key(KeyPress::new(
        KeyCode::Tab,
        Modifiers::default(),
    )));
    assert_eq!(left.messages, vec![Field::Commit]);
    assert!(interaction.text_target().is_none());
}
