//! 渲染的边界与确定性：空帧 / 零行 / 超长文本 / emoji 与组合字符 / 空键盘都不 panic；
//! 同一输入两次渲染输出逐字节一致；可跨线程共享的类型编译期断言 Send + Sync。

mod scenes;

use qingjian_render::{
    FontLibrary, Frame, Keyboard, KeyboardColors, Layout, PanelCell, PanelFrame, Rendered,
    Renderer, TextSizes, Theme,
};

fn renderer() -> Option<Renderer> {
    FontLibrary::system("zh-CN").ok().map(Renderer::new)
}

#[test]
fn same_input_renders_twice_byte_identical() {
    let Some(mut renderer) = renderer() else {
        eprintln!("没有系统字体，跳过");
        return;
    };
    let theme = Theme::light();
    let first = renderer
        .render(&scenes::nihao(), Layout::Vertical, &theme, 2.0)
        .unwrap();
    renderer.forget();
    let second = renderer
        .render(&scenes::nihao(), Layout::Vertical, &theme, 2.0)
        .unwrap();
    assert_eq!(first.pixmap.data(), second.pixmap.data());
    assert_eq!(
        (first.content_width, first.content_height),
        (second.content_width, second.content_height)
    );
}

#[test]
fn fresh_renderer_produces_the_same_output() {
    let (Some(mut a), Some(mut b)) = (renderer(), renderer()) else {
        eprintln!("没有系统字体，跳过");
        return;
    };
    let theme = Theme::light();
    let first = a
        .render(&scenes::nihao(), Layout::Vertical, &theme, 2.0)
        .unwrap();
    let second = b
        .render(&scenes::nihao(), Layout::Vertical, &theme, 2.0)
        .unwrap();
    assert_eq!(first.pixmap.data(), second.pixmap.data());
}

#[test]
fn panel_empty_frame_renders_without_panic() {
    let Some(mut renderer) = renderer() else {
        eprintln!("没有系统字体，跳过");
        return;
    };
    let theme = Theme::light();
    let rendered = renderer
        .render_panel(&PanelFrame::default(), &theme, 2.0)
        .unwrap();
    assert!(rendered.content_width > 0 && rendered.content_height > 0);
}

#[test]
fn panel_zero_rows_long_text_emoji_and_combining_char_render() {
    let Some(mut renderer) = renderer() else {
        eprintln!("没有系统字体，跳过");
        return;
    };
    let theme = Theme::light();
    let long = "超".repeat(200);
    let frame = PanelFrame {
        groups: vec!["搜索".to_owned()],
        sections: vec!["分类".to_owned()],
        cells: vec![
            Some(PanelCell {
                text: long,
                name: None,
            }),
            Some(PanelCell {
                text: "a".to_owned(),
                name: Some("单字".to_owned()),
            }),
            Some(PanelCell {
                text: "🙂あ̃".to_owned(),
                name: None,
            }),
        ],
        columns: 10,
        rows: 0,
        highlight: 0,
        ..PanelFrame::default()
    };
    let first = renderer.render_panel(&frame, &theme, 2.0).unwrap();
    renderer.forget();
    let second = renderer.render_panel(&frame, &theme, 2.0).unwrap();
    assert_eq!(
        (first.content_width, first.content_height),
        (second.content_width, second.content_height)
    );
}

#[test]
fn keyboard_empty_rows_renders_without_panic() {
    let Some(mut renderer) = renderer() else {
        eprintln!("没有系统字体，跳过");
        return;
    };
    let theme = Theme::light();
    let rendered = renderer
        .render_keyboard(&Keyboard::default(), &theme, 2.0, 1.0)
        .unwrap();
    assert!(rendered.pixmap.width() > 0 && rendered.pixmap.height() > 0);
}

#[test]
fn frame_with_zero_candidates_renders() {
    let Some(mut renderer) = renderer() else {
        eprintln!("没有系统字体，跳过");
        return;
    };
    let theme = Theme::light();
    let empty = Frame::default();
    let rendered = renderer
        .render(&empty, Layout::Vertical, &theme, 2.0)
        .unwrap();
    assert!(rendered.pixmap.width() > 0);
    // 单个字的极窄窗口也能画出来
    let narrow = Frame {
        rows: vec![qingjian_render::Row::plain(0, "a")],
        ..Frame::default()
    };
    let narrow = renderer
        .render(&narrow, Layout::Vertical, &theme, 2.0)
        .unwrap();
    assert!(narrow.pixmap.width() > 0);
}

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn shared_theme_and_result_types_are_send_sync() {
    assert_send_sync::<Theme>();
    assert_send_sync::<Rendered>();
    assert_send_sync::<KeyboardColors>();
    assert_send_sync::<TextSizes>();
}

#[test]
fn two_threads_render_independently_without_panic() {
    if FontLibrary::system("zh-CN").is_err() {
        eprintln!("没有系统字体，跳过");
        return;
    }
    let theme = Theme::light();
    let outputs: Vec<Vec<u8>> = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for _ in 0..2 {
            handles.push(scope.spawn(|| {
                let mut renderer = Renderer::new(FontLibrary::system("zh-CN").unwrap());
                let rendered = renderer
                    .render(&scenes::nihao(), Layout::Vertical, &theme, 2.0)
                    .unwrap();
                rendered.pixmap.data().to_vec()
            }));
        }
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect()
    });
    assert_eq!(outputs[0], outputs[1]);
}
