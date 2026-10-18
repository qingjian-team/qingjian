//! 循环动画的局部重画：同一时刻的动画帧要跟整张重画一模一样，不动的装饰（主题里伸出窗口的人物图等）不能丢。

mod scenes;

use std::time::Instant;

use qingjian_render::{FontLibrary, Layout, Renderer, Theme};

#[test]
fn loop_frame_matches_full_paint_at_the_same_moment() {
    let Ok(library) = FontLibrary::system("zh-CN") else {
        eprintln!("没有系统字体，跳过");
        return;
    };
    let mut renderer = Renderer::new(library);
    let frame = scenes::nihao();
    for theme in Theme::builtins() {
        for layout in [Layout::Vertical, Layout::Horizontal] {
            renderer.forget();
            let now = Instant::now();
            let full = renderer.render_at(&frame, layout, theme, 2.0, now).unwrap();
            let Some(ticked) = renderer.tick_at(now).unwrap() else {
                continue;
            };
            let differing = full
                .pixmap
                .pixels()
                .iter()
                .zip(ticked.pixmap.pixels())
                .filter(|(a, b)| a != b)
                .count();
            assert_eq!(differing, 0, "{} {layout:?}", theme.id());
        }
    }
}
