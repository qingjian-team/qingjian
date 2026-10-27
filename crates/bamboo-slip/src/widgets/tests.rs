use taffy::{FlexDirection, NodeId, Style};

use super::*;
use crate::fonts::FontLibrary;
use crate::scene::{Scene, Visual};
use crate::text::TextPainter;
use crate::ui::{Event, Interaction, Key, KeyCode, KeyPress, Modifiers, TextInput, Ui};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Pick(usize),
    Toggle,
    Down,
    Up,
}

/// 一排：分段选择（选中第 `selected` 个）、开关、到了上限的步进框。
fn frame(interaction: &mut Interaction<Message>, text: &mut TextPainter, selected: usize) {
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

fn layout(scene: &mut Scene, root: NodeId, text: &mut TextPainter) {
    scene.layout(root, text).unwrap();
}

fn press(interaction: &mut Interaction<Message>, key: KeyCode) -> Vec<Message> {
    interaction
        .handle(&Event::Key(KeyPress::new(key, Modifiers::default())))
        .messages
}

fn click(interaction: &mut Interaction<Message>, key: Key) -> Vec<Message> {
    let (x, y, width, height) = interaction.rect(key).unwrap();
    interaction.handle(&Event::PointerMoved {
        x: x + width / 2.0,
        y: y + height / 2.0,
    });
    interaction.handle(&Event::PointerDown);
    interaction.handle(&Event::PointerUp).messages
}

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

/// 上面空出 `above` 像素，下面一个开关、一个勾选框；交给舞台画，返回还有没有过渡在播。
fn shifted(stage: &mut crate::Stage, above: f32, on: bool, now: std::time::Instant) -> bool {
    let theme = WidgetTheme::new(WidgetColors::LIGHT, 1.0);
    let mut interaction = Interaction::<Message>::new();
    let mut ui = Ui::new(&interaction);
    let spacer = ui
        .scene()
        .node(
            Style {
                size: taffy::Size {
                    width: taffy::Dimension::length(10.0),
                    height: taffy::Dimension::length(above),
                },
                ..Style::default()
            },
            Visual::Group,
            &[],
        )
        .unwrap();
    let switch = Switch::new(on)
        .build(&mut ui, &theme, "switch", Message::Toggle)
        .unwrap();
    let checkbox = Checkbox::new(on, "勾")
        .build(&mut ui, &theme, "check", Message::Toggle)
        .unwrap();
    let root = ui
        .scene()
        .node(
            Style {
                flex_direction: FlexDirection::Column,
                ..Style::default()
            },
            Visual::Group,
            &[spacer, switch, checkbox],
        )
        .unwrap();
    let (mut scene, elements) = ui.finish();
    let size = scene.layout(root, stage.text()).unwrap();
    interaction.attach(&scene, root, elements).unwrap();
    stage
        .present(scene, root, size, 0, 1.0, now)
        .unwrap()
        .next_frame
        .is_some()
}

#[test]
fn controls_pushed_by_layout_jump_while_their_own_parts_still_slide() {
    let mut stage = crate::Stage::new(FontLibrary::system("en").unwrap());
    let start = std::time::Instant::now();
    assert!(!shifted(&mut stage, 0.0, false, start));
    let later = start + std::time::Duration::from_secs(1);
    assert!(
        !shifted(&mut stage, 40.0, false, later),
        "整体被挤下去不该有过渡"
    );
    assert!(shifted(&mut stage, 40.0, true, later), "拨开关时圆钮要滑");
}

#[test]
fn rows_claim_the_words_on_their_own_controls() {
    let theme = WidgetTheme::new(WidgetColors::LIGHT, 1.0);
    let interaction = Interaction::<Message>::new();
    let mut ui = Ui::new(&interaction);
    // 两行的控件都先建好、再建行：各行只收自己控件上的字
    let first = Segmented::new(&["一", "二"], 0)
        .build(&mut ui, &theme, "first", Message::Pick)
        .unwrap();
    let second = Select::new("英语")
        .options(&["英语", "日语"])
        .build(&mut ui, &theme, "second", Message::Toggle)
        .unwrap();
    SettingRow::new("数字")
        .keywords(&["数目"])
        .control(first)
        .build(&mut ui, &theme)
        .unwrap();
    SettingRow::new("译文")
        .control(second)
        .build(&mut ui, &theme)
        .unwrap();
    let found = ui.take_searchables();
    assert_eq!(found[0].keywords, ["一", "二", "数目"]);
    assert_eq!(found[1].keywords, ["英语", "英语", "日语"]);
}

#[test]
fn info_marks_with_a_link_send_it_when_clicked() {
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    let mut interaction = Interaction::<Message>::new();
    let theme = WidgetTheme::new(WidgetColors::LIGHT, 1.0);
    let mut ui = Ui::new(&interaction);
    ui.info_links("点击查看文档", |title| {
        (title == "有链接").then_some(Message::Toggle)
    });
    let linked = SettingRow::new("有链接")
        .info("说明")
        .build(&mut ui, &theme)
        .unwrap();
    let plain = SettingRow::new("没链接")
        .info("说明")
        .build(&mut ui, &theme)
        .unwrap();
    let root = ui
        .scene()
        .node(
            Style {
                flex_direction: FlexDirection::Column,
                ..Style::default()
            },
            Visual::Group,
            &[linked, plain],
        )
        .unwrap();
    let (mut scene, elements) = ui.finish();
    layout(&mut scene, root, &mut text);
    interaction.attach(&scene, root, elements).unwrap();
    assert_eq!(
        click(&mut interaction, Key::ROOT.child(("info", "有链接"))),
        [Message::Toggle]
    );
    assert!(click(&mut interaction, Key::ROOT.child(("info", "没链接"))).is_empty());
}

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
                align_items: taffy::AlignItems::FLEX_START,
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

#[test]
fn fitting_text_cuts_it_and_adds_an_ellipsis() {
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    let theme = WidgetTheme::new(WidgetColors::LIGHT, 1.0);
    let style = theme.text(13.0, 400, theme.colors.text);
    let whole = "五笔测试（不自动上屏）";
    let width = text.measure(whole, &style).width;
    assert_eq!(text.fit(whole, &style, width), whole);
    let fitted = text.fit(whole, &style, width / 2.0);
    assert!(
        fitted.ends_with('…') && fitted.len() < whole.len(),
        "{fitted}"
    );
    assert!(text.measure(&fitted, &style).width <= width / 2.0 + 0.5);
    assert_eq!(text.fit(whole, &style, 1.0), "…");
}

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
