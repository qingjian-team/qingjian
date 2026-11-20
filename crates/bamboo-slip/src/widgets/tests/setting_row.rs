use taffy::{FlexDirection, Style};

use super::super::*;
use super::Message;
use super::{click, layout};
use crate::fonts::FontLibrary;
use crate::scene::Visual;
use crate::text::TextPainter;
use crate::ui::{Interaction, Key, Ui};

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
