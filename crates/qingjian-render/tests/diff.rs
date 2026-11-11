//! 次选里和首选不一样的字：带差异范围的候选按主题的 `diff` 颜色画那几个字，其余照旧。

use qingjian_render::{FontLibrary, Frame, Layout, Mode, Preedit, Rendered, Renderer, Row, Theme};

const THEME: &str = r##"{
  "extends": "qingjian",
  "schema": 1,
  "meta": { "id": "diff-test", "name": "测试", "author": "测试", "license": "MIT" },
  "variables": { "diff": "#ff0000", "hl_diff": "#ff0000" }
}"##;

#[allow(clippy::single_range_in_vec_init)]
fn frame(diff: bool) -> Frame {
    let mut other = Row::plain(1, "今天天汽不错");
    if diff {
        other.diff = vec![9..12];
    }
    Frame {
        preedit: Some(Preedit::plain("jintiantianqibucuo", 18)),
        rows: vec![Row::plain(0, "今天天气不错"), other],
        highlighted: Some(0),
        footer: None,
        previous_page: false,
        next_page: false,
        sentence: None,
        status: None,
        mode: Mode::default(),
        columns: 0,
        column_ems: Vec::new(),
        reversed: false,
    }
}

/// 偏红的像素数（红远大于绿蓝，不透明）。
fn reddish(rendered: &Rendered) -> usize {
    rendered
        .pixmap
        .data()
        .chunks_exact(4)
        .filter(|p| p[3] > 200 && p[0] > 180 && p[1] < 90 && p[2] < 90)
        .count()
}

#[test]
fn differing_characters_take_the_diff_color() {
    let Ok(library) = FontLibrary::system("zh-CN") else {
        eprintln!("没有系统字体，跳过");
        return;
    };
    let theme = Theme::from_json(THEME, false).unwrap();
    let mut renderer = Renderer::new(library);
    let plain = renderer
        .render(&frame(false), Layout::Vertical, &theme, 2.0)
        .unwrap();
    let marked = renderer
        .render(&frame(true), Layout::Vertical, &theme, 2.0)
        .unwrap();
    assert_eq!(reddish(&plain), 0);
    assert!(reddish(&marked) > 20, "「汽」画成了红色");
    assert_eq!(
        (plain.pixmap.width(), plain.pixmap.height()),
        (marked.pixmap.width(), marked.pixmap.height()),
        "换色不改排版"
    );
}
