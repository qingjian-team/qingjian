use taffy::{Dimension, Size, Style};

use super::super::*;
use super::Message;
use crate::fonts::FontLibrary;
use crate::scene::Visual;
use crate::text::TextPainter;
use crate::ui::{Event, Interaction, Key, Ui};

/// 200 见方的视口里 `content_height` 高的内容。
fn frame_scroll(
    interaction: &mut Interaction<Message>,
    text: &mut TextPainter,
    content_height: f32,
) {
    let theme = WidgetTheme::new(WidgetColors::LIGHT, 1.0);
    let mut ui = Ui::new(interaction);
    let content = ui
        .scene()
        .node(
            Style {
                size: Size {
                    width: Dimension::length(180.0),
                    height: Dimension::length(content_height),
                },
                flex_shrink: 0.0,
                ..Style::default()
            },
            Visual::Group,
            &[],
        )
        .unwrap();
    let root = ScrollArea::new(Style {
        size: Size {
            width: Dimension::length(200.0),
            height: Dimension::length(200.0),
        },
        ..Style::default()
    })
    .build(&mut ui, &theme, "list", content)
    .unwrap();
    let (mut scene, elements) = ui.finish();
    scene.layout(root, text).unwrap();
    interaction.attach(&scene, root, elements).unwrap();
}

/// 200 见方的视口里 800 高的内容。
fn scroller(interaction: &mut Interaction<Message>, text: &mut TextPainter) {
    frame_scroll(interaction, text, 800.0);
}

#[test]
fn scrollbar_drags_and_jumps() {
    let mut interaction = Interaction::new();
    let mut text = TextPainter::new(FontLibrary::system("zh-CN").unwrap());
    let list = Key::ROOT.child("list");
    // 第二帧才知道内容多高，才有滚动条
    scroller(&mut interaction, &mut text);
    scroller(&mut interaction, &mut text);
    let offset = |interaction: &Interaction<Message>| interaction.scroll_offset(list).1;
    let near = |actual: f32, expected: f32| {
        assert!((actual - expected).abs() < 0.5, "{actual} ≠ {expected}");
    };
    // 滑块长 48.5、能走 145.5，内容能滚 600。按在滑块上不跳
    interaction.handle(&Event::PointerMoved { x: 195.0, y: 13.0 });
    interaction.handle(&Event::PointerDown);
    near(offset(&interaction), 0.0);
    let moved = interaction.handle(&Event::PointerMoved { x: 195.0, y: 85.75 });
    assert!(moved.changed);
    near(offset(&interaction), 300.0);
    // 拖到元素外面也跟着，超出的夹到头
    interaction.handle(&Event::PointerMoved {
        x: -500.0,
        y: 900.0,
    });
    near(offset(&interaction), 600.0);
    interaction.handle(&Event::PointerUp);
    // 抬起之后指针怎么动都不滚
    interaction.handle(&Event::PointerMoved { x: 100.0, y: 10.0 });
    near(offset(&interaction), 600.0);
    // 点轨道：滑块中间跳到指针处
    scroller(&mut interaction, &mut text);
    interaction.handle(&Event::PointerMoved { x: 195.0, y: 80.0 });
    interaction.handle(&Event::PointerDown);
    near(offset(&interaction), (80.0 - 24.25 - 3.0) / 145.5 * 600.0);
    interaction.handle(&Event::PointerUp);
    // 视口其余地方按下不算
    let before = offset(&interaction);
    interaction.handle(&Event::PointerMoved { x: 100.0, y: 150.0 });
    interaction.handle(&Event::PointerDown);
    interaction.handle(&Event::PointerMoved { x: 100.0, y: 50.0 });
    near(offset(&interaction), before);
}

#[test]
fn short_content_has_no_scrollbar_and_stays_put() {
    let mut interaction = Interaction::new();
    let mut text = TextPainter::new(FontLibrary::system("zh-CN").unwrap());
    let list = Key::ROOT.child("list");
    frame_scroll(&mut interaction, &mut text, 100.0);
    frame_scroll(&mut interaction, &mut text, 100.0);
    assert!(
        interaction
            .scroll_extent(list)
            .is_none_or(|extent| extent.max().1 == 0.0)
    );
    assert_eq!(interaction.scroll_offset(list), (0.0, 0.0));
    assert!(interaction.rect(list.child("track")).is_none());
    interaction.handle(&Event::Scroll { dx: 0.0, dy: 50.0 });
    assert_eq!(interaction.scroll_offset(list), (0.0, 0.0));
}

#[test]
fn dragged_offset_clamps_to_content_max() {
    let mut interaction = Interaction::new();
    let mut text = TextPainter::new(FontLibrary::system("zh-CN").unwrap());
    let list = Key::ROOT.child("list");
    scroller(&mut interaction, &mut text);
    scroller(&mut interaction, &mut text);
    let extent = interaction.scroll_extent(list).unwrap();
    interaction.handle(&Event::PointerMoved { x: 195.0, y: 13.0 });
    interaction.handle(&Event::PointerDown);
    interaction.handle(&Event::PointerMoved {
        x: -500.0,
        y: 900.0,
    });
    interaction.handle(&Event::PointerUp);
    let offset = interaction.scroll_offset(list).1;
    assert!(offset >= 0.0 && offset <= extent.max().1 + 0.5, "{offset}");
}
