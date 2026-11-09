//! 手机候选条：候选排成一行、没有投影边，拼音行在候选上面，壳按 `candidates_top` 分界钉住拼音行；各内置主题都要能画。

mod scenes;

use qingjian_render::{FontLibrary, HitTarget, Layout, Renderer, Theme};

fn renderer() -> Option<Renderer> {
    match FontLibrary::system("zh-CN") {
        Ok(library) => Some(Renderer::new(library)),
        Err(_) => {
            eprintln!("没有系统字体，跳过");
            None
        }
    }
}

#[test]
fn candidates_line_up_under_the_pinyin_line() {
    let Some(mut renderer) = renderer() else {
        return;
    };
    let frame = scenes::nihao();
    for theme in Theme::builtins() {
        let what = theme.id();
        let rendered = renderer.render(&frame, Layout::Strip, theme, 2.0).unwrap();
        assert_eq!(
            (rendered.content_x, rendered.content_y),
            (0, 0),
            "{what}：候选条不该有投影边"
        );
        let cells: Vec<_> = (0..frame.rows.len())
            .map(|i| {
                *rendered
                    .hits
                    .iter()
                    .find(|region| region.target == HitTarget::Candidate(i))
                    .unwrap_or_else(|| panic!("{what}：没有第 {i} 个候选的点击区域"))
            })
            .collect();
        for pair in cells.windows(2) {
            assert_eq!(pair[0].y, pair[1].y, "{what}：候选不在同一行");
            assert!(
                pair[1].x >= pair[0].x + pair[0].width,
                "{what}：候选重叠或倒序"
            );
        }
        let top = rendered.candidates_top();
        assert!(
            top > 0 && top < rendered.pixmap.height(),
            "{what}：拼音行与候选的分界 {top} 不在位图里"
        );
        let first = cells[0];
        for region in &rendered.hits {
            if let HitTarget::Translation { candidate: 0, .. } = region.target {
                assert!(
                    region.y >= first.y && region.x >= first.x - 0.5,
                    "{what}：译文没落在候选格里"
                );
            }
        }
        assert!(
            theme.variable("surface").is_some(),
            "{what}：没有 surface 变量"
        );
    }
}

#[test]
fn without_pinyin_the_candidates_start_at_the_top() {
    let Some(mut renderer) = renderer() else {
        return;
    };
    let mut frame = scenes::nihao();
    frame.preedit = None;
    let rendered = renderer
        .render(&frame, Layout::Strip, &Theme::light(), 2.0)
        .unwrap();
    assert_eq!(rendered.candidates_top(), 0);
}
