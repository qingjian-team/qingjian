//! 状态条的节点树画法：格子按 `repeat` 绑定 `cells` 展开，点击边界按各格盒子算；输入状态能用 `when` 分支。

use std::path::PathBuf;

use qingjian_render::{FontLibrary, Mode, Renderer, StatusCell, Theme};

/// 左边按中英各画一个同尺寸不同色的方块（不用字：Linux CI 没有中文字体，两个字都画成同一个缺字框），
/// 格子之间各夹一个 10 点宽的装饰，格子 4 点内边距。
const THEME: &str = r##"{
    "extends": "qingjian", "schema": 1, "meta": { "id": "status-tree", "name": "status-tree" },
    "components": {
        "cell": { "type": "frame", "padding": 4, "children": [
            { "type": "text", "bind": "text", "font": "candidate", "color": "@text" },
            { "type": "icon", "icon": "gear", "when": "gear", "size": 12, "color": "@gloss" }
        ] },
        "status-item": { "type": "frame", "children": [
            { "type": "frame", "width": 10, "height": 10, "when": "!first", "fill": { "image": "images/dot.svg" } },
            { "type": "use", "component": "cell" }
        ] }
    },
    "status": { "root": { "type": "frame", "fill": "@surface", "children": [
        { "type": "frame", "width": 16, "height": 16, "when": "mode.english", "fill": "#0000ff" },
        { "type": "frame", "width": 16, "height": 16, "when": "!mode.english", "fill": "#00ff00" },
        { "type": "repeat", "bind": "cells", "component": "status-item" }
    ] } }
}"##;

const DOT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><circle cx="5" cy="5" r="5" fill="#ff0000"/></svg>"##;

fn theme() -> Theme {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("status-tree");
    std::fs::create_dir_all(dir.join("images")).unwrap();
    std::fs::write(dir.join("images/dot.svg"), DOT).unwrap();
    std::fs::write(dir.join("theme.json"), THEME).unwrap();
    Theme::from_dir(&dir, false).unwrap()
}

#[test]
fn status_tree_cells_and_mode() {
    let Ok(library) = FontLibrary::system("zh-CN") else {
        eprintln!("没有系统字体，跳过");
        return;
    };
    let mut renderer = Renderer::new(library);
    let theme = theme();
    let cells = [
        StatusCell::text("中 · 小鹤", true),
        StatusCell::text(",.", false),
        StatusCell::Gear,
    ];
    let chinese = renderer
        .render_status(&cells, &Mode::default(), &theme, 2.0)
        .unwrap();
    let edges = &chinese.cell_edges;
    assert_eq!(edges.len(), 3);
    assert!(edges.windows(2).all(|pair| pair[0] < pair[1]), "{edges:?}");
    assert_eq!(
        *edges.last().unwrap(),
        chinese.rendered.content_width as f32
    );
    // 第一格前面是中文模式的方块：第一格的右边界比它自己的宽度靠右
    assert!(edges[0] > 0.0);

    // 第二格前的红点装饰画出来了：在第一格右边界之后 10 点（20 像素）以内找红色
    let pixmap = &chinese.rendered.pixmap;
    let (cx, cy) = (chinese.rendered.content_x, chinese.rendered.content_y);
    let y = cy + 10;
    let red = (edges[0] as u32..edges[0] as u32 + 20).any(|x| {
        let pixel = pixmap.pixel(cx + x, y).unwrap().demultiply();
        pixel.red() > 200 && pixel.green() < 60
    });
    assert!(red, "格子之间的装饰没画出来");

    // 英文模式左边换成另一色的方块：同宽，格子边界不变
    let english = renderer
        .render_status(
            &cells,
            &Mode {
                english: true,
                ..Mode::default()
            },
            &theme,
            2.0,
        )
        .unwrap();
    assert_eq!(english.cell_edges, chinese.cell_edges);
    assert_ne!(
        english.rendered.pixmap.data(),
        chinese.rendered.pixmap.data()
    );
}
