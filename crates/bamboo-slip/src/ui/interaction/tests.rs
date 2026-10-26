use taffy::{Dimension, FlexDirection, LengthPercentage, NodeId, Rect, Size, Style};

use super::*;
use crate::fonts::FontLibrary;
use crate::scene::Visual;
use crate::text::TextPainter;
use crate::ui::{Interact, KeyCode, KeyPress, Modifiers, Ui};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Ok,
    Cancel,
    Close,
    /// 拖动到的位置（百分比）。
    At(u8),
}

/// 200×100 的根（登记了 Esc）里左右两个按钮：左边「确定」里有个图标，右边「取消」按 `cancel_disabled` 禁用。
fn frame(interaction: &mut Interaction<Message>, cancel_disabled: bool) -> bool {
    let mut ui = Ui::new(interaction);
    let fixed = |width: f32, height: f32| Style {
        size: Size {
            width: Dimension::length(width),
            height: Dimension::length(height),
        },
        ..Style::default()
    };
    let icon = ui
        .scene()
        .node(fixed(20.0, 20.0), Visual::Group, &[])
        .unwrap();
    let icon_key = ui.key("icon");
    ui.interact(icon, icon_key, Interact::new());
    let ok = ui
        .scene()
        .node(
            Style {
                padding: Rect {
                    left: LengthPercentage::length(10.0),
                    right: LengthPercentage::length(10.0),
                    top: LengthPercentage::length(10.0),
                    bottom: LengthPercentage::length(10.0),
                },
                ..fixed(100.0, 100.0)
            },
            Visual::Group,
            &[icon],
        )
        .unwrap();
    let ok_key = ui.key("ok");
    ui.interact(
        ok,
        ok_key,
        Interact::new().on_click(Message::Ok).focusable(),
    );
    let cancel = ui
        .scene()
        .node(fixed(100.0, 100.0), Visual::Group, &[])
        .unwrap();
    let cancel_key = ui.key("cancel");
    ui.interact(
        cancel,
        cancel_key,
        Interact::new()
            .on_click(Message::Cancel)
            .focusable()
            .disabled(cancel_disabled),
    );
    let root = ui
        .scene()
        .node(
            Style {
                flex_direction: FlexDirection::Row,
                ..Style::default()
            },
            Visual::Group,
            &[ok, cancel],
        )
        .unwrap();
    let root_key = ui.key("root");
    ui.interact(
        root,
        root_key,
        Interact::new().on_key(|press| (press.key == KeyCode::Escape).then_some(Message::Close)),
    );
    let (mut scene, elements) = ui.finish();
    layout(&mut scene, root);
    interaction.attach(&scene, root, elements).unwrap()
}

fn layout(scene: &mut Scene, root: NodeId) {
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    scene.layout(root, &mut text).unwrap();
}

fn key(name: &str) -> Key {
    Key::ROOT.child(name)
}

fn press(code: KeyCode, shift: bool) -> Event {
    Event::Key(KeyPress::new(
        code,
        Modifiers {
            shift,
            ..Modifiers::default()
        },
    ))
}

#[test]
fn hover_covers_target_and_ancestors() {
    let mut interaction = Interaction::new();
    frame(&mut interaction, false);
    let response = interaction.handle(&Event::PointerMoved { x: 15.0, y: 15.0 });
    assert!(response.changed);
    assert!(interaction.state(key("icon")).hovered);
    assert!(interaction.state(key("ok")).hovered);
    assert!(interaction.state(key("root")).hovered);
    assert!(!interaction.state(key("cancel")).hovered);

    let response = interaction.handle(&Event::PointerMoved { x: 50.0, y: 50.0 });
    assert!(response.changed);
    assert!(!interaction.state(key("icon")).hovered);
    let response = interaction.handle(&Event::PointerMoved { x: 51.0, y: 50.0 });
    assert!(!response.changed);
}

#[test]
fn click_bubbles_to_nearest_handler() {
    let mut interaction = Interaction::new();
    frame(&mut interaction, false);
    interaction.handle(&Event::PointerMoved { x: 15.0, y: 15.0 });
    interaction.handle(&Event::PointerDown);
    assert!(interaction.state(key("ok")).pressed);
    assert!(interaction.state(key("ok")).focused);
    assert!(!interaction.state(key("ok")).focus_ring);
    let response = interaction.handle(&Event::PointerUp);
    assert_eq!(response.messages, vec![Message::Ok]);
}

#[test]
fn dragging_out_cancels_click() {
    let mut interaction = Interaction::new();
    frame(&mut interaction, false);
    interaction.handle(&Event::PointerMoved { x: 50.0, y: 50.0 });
    interaction.handle(&Event::PointerDown);
    interaction.handle(&Event::PointerMoved { x: 150.0, y: 50.0 });
    assert!(!interaction.state(key("ok")).pressed);
    let response = interaction.handle(&Event::PointerUp);
    assert!(response.messages.is_empty());
}

#[test]
fn disabled_element_swallows_click() {
    let mut interaction = Interaction::new();
    frame(&mut interaction, true);
    interaction.handle(&Event::PointerMoved { x: 150.0, y: 50.0 });
    interaction.handle(&Event::PointerDown);
    let response = interaction.handle(&Event::PointerUp);
    assert!(response.messages.is_empty());
}

#[test]
fn tab_cycles_focus_and_space_clicks() {
    let mut interaction = Interaction::new();
    frame(&mut interaction, false);
    interaction.handle(&press(KeyCode::Tab, false));
    assert!(interaction.state(key("ok")).focus_ring);
    interaction.handle(&press(KeyCode::Tab, false));
    assert!(interaction.state(key("cancel")).focus_ring);
    interaction.handle(&press(KeyCode::Tab, false));
    assert!(interaction.state(key("ok")).focus_ring);
    interaction.handle(&press(KeyCode::Tab, true));
    assert!(interaction.state(key("cancel")).focused);

    let response = interaction.handle(&press(KeyCode::Space, false));
    assert_eq!(response.messages, vec![Message::Cancel]);
    let response = interaction.handle(&press(KeyCode::Escape, false));
    assert_eq!(response.messages, vec![Message::Close]);
}

#[test]
fn tab_skips_disabled() {
    let mut interaction = Interaction::new();
    frame(&mut interaction, true);
    interaction.handle(&press(KeyCode::Tab, false));
    interaction.handle(&press(KeyCode::Tab, false));
    assert!(interaction.state(key("ok")).focused);
}

#[test]
fn escape_without_focus_reaches_root() {
    let mut interaction = Interaction::new();
    frame(&mut interaction, false);
    let response = interaction.handle(&press(KeyCode::Escape, false));
    assert_eq!(response.messages, vec![Message::Close]);
}

#[test]
fn state_survives_rebuild_and_lost_focus_clears() {
    let mut interaction = Interaction::new();
    frame(&mut interaction, false);
    interaction.handle(&Event::PointerMoved { x: 150.0, y: 50.0 });
    interaction.handle(&press(KeyCode::Tab, true));
    assert!(interaction.state(key("cancel")).focused);

    assert!(!frame(&mut interaction, false));
    assert!(interaction.state(key("cancel")).hovered);
    assert!(interaction.state(key("cancel")).focused);

    // 禁用后失去焦点
    frame(&mut interaction, true);
    interaction.handle(&press(KeyCode::Tab, false));
    assert!(interaction.state(key("ok")).focused);
}

/// 100×100 的视口（裁剪）里一列 300 高的内容，内容里一个 100×50 的按钮在最上面；`offset` 是滚动位置。
fn scrolling(interaction: &mut Interaction<Message>) {
    let offset = interaction.scroll_offset(key("viewport")).1;
    let mut ui = Ui::new(interaction);
    let fixed = |width: f32, height: f32| Style {
        size: Size {
            width: Dimension::length(width),
            height: Dimension::length(height),
        },
        flex_shrink: 0.0,
        ..Style::default()
    };
    let button = ui
        .scene()
        .node(fixed(100.0, 50.0), Visual::Group, &[])
        .unwrap();
    let button_key = ui.key("ok");
    ui.interact(button, button_key, Interact::new().on_click(Message::Ok));
    let content = ui
        .scene()
        .node(
            Style {
                flex_direction: FlexDirection::Column,
                position: taffy::Position::Relative,
                inset: taffy::Rect {
                    top: taffy::LengthPercentageAuto::length(-offset),
                    ..taffy::Rect::auto()
                },
                ..fixed(100.0, 300.0)
            },
            Visual::Group,
            &[button],
        )
        .unwrap();
    let viewport = ui
        .scene()
        .node(
            Style {
                flex_direction: FlexDirection::Column,
                overflow: taffy::Point {
                    x: taffy::Overflow::Hidden,
                    y: taffy::Overflow::Hidden,
                },
                ..fixed(100.0, 100.0)
            },
            Visual::Group,
            &[content],
        )
        .unwrap();
    ui.scene().set_clip(viewport, 0.0);
    let viewport_key = ui.key("viewport");
    ui.interact(viewport, viewport_key, Interact::new().scrolls(content));
    let (mut scene, elements) = ui.finish();
    layout(&mut scene, viewport);
    interaction.attach(&scene, viewport, elements).unwrap();
}

#[test]
fn scroll_clamps_to_content_and_clip_hides_scrolled_out_parts() {
    let mut interaction = Interaction::new();
    scrolling(&mut interaction);
    let extent = interaction.scroll_extent(key("viewport")).unwrap();
    assert_eq!(extent.max(), (0.0, 200.0));

    interaction.handle(&Event::PointerMoved { x: 50.0, y: 80.0 });
    let response = interaction.handle(&Event::Scroll { dx: 0.0, dy: 30.0 });
    assert!(response.changed);
    scrolling(&mut interaction);
    assert_eq!(interaction.scroll_offset(key("viewport")), (0.0, 30.0));
    // 按钮被挪到 -30..20：视口里看得见的那段还能点
    interaction.handle(&Event::PointerMoved { x: 50.0, y: 10.0 });
    assert!(interaction.state(key("ok")).hovered);
    // 按钮盒子伸到视口上面的那段被裁掉了，点不到
    interaction.handle(&Event::PointerMoved { x: 50.0, y: -10.0 });
    assert!(!interaction.state(key("ok")).hovered);
    // 指针不在视口上时滚不动
    assert!(
        !interaction
            .handle(&Event::Scroll { dx: 0.0, dy: 10.0 })
            .changed
    );

    interaction.handle(&Event::PointerMoved { x: 50.0, y: 50.0 });
    interaction.handle(&Event::Scroll {
        dx: 0.0,
        dy: 1000.0,
    });
    scrolling(&mut interaction);
    assert_eq!(interaction.scroll_offset(key("viewport")), (0.0, 200.0));
    interaction.handle(&Event::PointerMoved { x: 50.0, y: 50.0 });
    assert!(
        !interaction
            .handle(&Event::Scroll { dx: 0.0, dy: 10.0 })
            .changed
    );
}

/// 100 高的视口里一列内容：上面 250 空着，再下面是 30 高的 `far`。
fn far_below(interaction: &mut Interaction<Message>) {
    let offset = interaction.scroll_offset(key("viewport")).1;
    let mut ui = Ui::new(interaction);
    let fixed = |width: f32, height: f32| Style {
        size: Size {
            width: Dimension::length(width),
            height: Dimension::length(height),
        },
        flex_shrink: 0.0,
        ..Style::default()
    };
    let spacer = ui
        .scene()
        .node(fixed(100.0, 250.0), Visual::Group, &[])
        .unwrap();
    let far = ui
        .scene()
        .node(fixed(100.0, 30.0), Visual::Group, &[])
        .unwrap();
    let far_key = ui.key("far");
    ui.interact(far, far_key, Interact::new());
    let content = ui
        .scene()
        .node(
            Style {
                flex_direction: FlexDirection::Column,
                position: taffy::Position::Relative,
                inset: taffy::Rect {
                    top: taffy::LengthPercentageAuto::length(-offset),
                    ..taffy::Rect::auto()
                },
                ..fixed(100.0, 300.0)
            },
            Visual::Group,
            &[spacer, far],
        )
        .unwrap();
    let viewport = ui
        .scene()
        .node(fixed(100.0, 100.0), Visual::Group, &[content])
        .unwrap();
    let viewport_key = ui.key("viewport");
    ui.interact(viewport, viewport_key, Interact::new().scrolls(content));
    let (mut scene, elements) = ui.finish();
    layout(&mut scene, viewport);
    interaction.attach(&scene, viewport, elements).unwrap();
}

#[test]
fn reveal_scrolls_just_enough_and_only_once() {
    let mut interaction = Interaction::new();
    far_below(&mut interaction);
    assert!(interaction.reveal(key("far"), 0.0));
    assert_eq!(interaction.scroll_offset(key("viewport")), (0.0, 180.0));
    far_below(&mut interaction);
    // 已经在视野里：不动
    assert!(!interaction.reveal(key("far"), 0.0));
}

/// 一条 200×20 的拖动条，左边空出 50 像素。
fn track(interaction: &mut Interaction<Message>) {
    let mut ui = Ui::new(interaction);
    let bar = ui
        .scene()
        .node(
            Style {
                size: Size {
                    width: Dimension::length(200.0),
                    height: Dimension::length(20.0),
                },
                ..Style::default()
            },
            Visual::Group,
            &[],
        )
        .unwrap();
    let bar_key = ui.key("bar");
    ui.interact(
        bar,
        bar_key,
        Interact::new().on_drag(|fraction| Some(Message::At((fraction * 100.0).round() as u8))),
    );
    let root = ui
        .scene()
        .node(
            Style {
                padding: Rect {
                    left: LengthPercentage::length(50.0),
                    right: LengthPercentage::length(0.0),
                    top: LengthPercentage::length(0.0),
                    bottom: LengthPercentage::length(0.0),
                },
                ..Style::default()
            },
            Visual::Group,
            &[bar],
        )
        .unwrap();
    let (mut scene, elements) = ui.finish();
    layout(&mut scene, root);
    interaction.attach(&scene, root, elements).unwrap();
}

#[test]
fn drag_reports_position_until_release_even_outside() {
    let mut interaction = Interaction::new();
    track(&mut interaction);
    interaction.handle(&Event::PointerMoved { x: 100.0, y: 10.0 });
    let down = interaction.handle(&Event::PointerDown);
    assert_eq!(down.messages, vec![Message::At(25)]);
    let moved = interaction.handle(&Event::PointerMoved { x: 400.0, y: 80.0 });
    assert_eq!(moved.messages, vec![Message::At(100)]);
    let up = interaction.handle(&Event::PointerUp);
    assert!(up.messages.is_empty());
    let after = interaction.handle(&Event::PointerMoved { x: 60.0, y: 10.0 });
    assert!(after.messages.is_empty());
}
