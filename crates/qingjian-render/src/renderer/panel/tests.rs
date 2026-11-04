use crate::panel::{PanelCell, PanelFrame, PanelTranslate};
use crate::renderer::{HitTarget, PanelHit, Renderer};
use crate::theme::Theme;
use bamboo_slip::FontLibrary;

fn frame(sections: usize, cells: usize, columns: usize) -> PanelFrame {
    PanelFrame {
        groups: vec!["搜索".into(), "符号".into(), "表情".into()],
        group: 1,
        sections: (0..sections).map(|i| format!("分类{i}")).collect(),
        section: sections.saturating_sub(1),
        cells: (0..cells)
            .map(|i| {
                Some(PanelCell {
                    text: format!("{}", char::from_u32(0x2190 + i as u32).unwrap()),
                    name: Some("箭头".into()),
                })
            })
            .collect(),
        columns,
        rows: 6,
        highlight: 0,
        footer: Some("1/3".into()),
        hint: "Tab 换组".into(),
        translate: None,
    }
}

#[test]
fn size_does_not_depend_on_content_and_hits_cover_every_part() {
    // 没有系统字体的环境（CI 容器）跳过
    let Ok(library) = FontLibrary::system("zh-CN") else {
        return;
    };
    let mut renderer = Renderer::new(library);
    let theme = Theme::light();
    let full = renderer
        .render_panel(&frame(30, 60, 10), &theme, 2.0)
        .unwrap();
    let sparse = renderer.render_panel(&frame(2, 3, 4), &theme, 2.0).unwrap();
    assert_eq!(
        (full.content_width, full.content_height),
        (sparse.content_width, sparse.content_height)
    );
    let count = |rendered: &crate::Rendered, f: fn(&HitTarget) -> bool| {
        rendered.hits.iter().filter(|hit| f(&hit.target)).count()
    };
    assert_eq!(
        count(&full, |t| matches!(t, HitTarget::Panel(PanelHit::Group(_)))),
        3
    );
    assert_eq!(
        count(&full, |t| matches!(t, HitTarget::Panel(PanelHit::Close))),
        1
    );
    assert_eq!(
        count(&full, |t| matches!(t, HitTarget::Panel(PanelHit::Cell(_)))),
        60
    );
    assert_eq!(
        count(&sparse, |t| matches!(
            t,
            HitTarget::Panel(PanelHit::Cell(_))
        )),
        3
    );
    // 三十个分类放不下，只露出一部分，当前（最后一个）一定在里面
    let sections: Vec<usize> = full
        .hits
        .iter()
        .filter_map(|hit| match hit.target {
            HitTarget::Panel(PanelHit::Section(i)) => Some(i),
            _ => None,
        })
        .collect();
    assert!(sections.len() < 30 && sections.contains(&29));
    assert_eq!(
        count(&sparse, |t| matches!(
            t,
            HitTarget::Panel(PanelHit::Section(_))
        )),
        2
    );
}

#[test]
fn translate_box_keeps_the_size_and_hits_its_candidates() {
    let Ok(library) = FontLibrary::system("zh-CN") else {
        return;
    };
    let mut renderer = Renderer::new(library);
    let theme = Theme::light();
    let grid = renderer
        .render_panel(&frame(5, 60, 10), &theme, 2.0)
        .unwrap();
    let boxed = PanelFrame {
        translate: Some(PanelTranslate {
            source: "今天天气很好，我们去公园散步吧。".repeat(20),
            cursor: 3,
            selection: None,
            placeholder: String::new(),
            composing: "nihao".into(),
            candidates: vec!["你好".into(), "拟好".into(), "泥".into()],
            highlight: 1,
            status: "译成英语 · 翻译中…".into(),
        }),
        ..frame(5, 0, 10)
    };
    let boxed = renderer.render_panel(&boxed, &theme, 2.0).unwrap();
    assert_eq!(
        (grid.content_width, grid.content_height),
        (boxed.content_width, boxed.content_height)
    );
    let candidates: Vec<usize> = boxed
        .hits
        .iter()
        .filter_map(|hit| match hit.target {
            HitTarget::Panel(PanelHit::Candidate(i)) => Some(i),
            _ => None,
        })
        .collect();
    assert_eq!(candidates, [0, 1, 2]);
    // 原文能点：落到各个字的边界上，光标在前面时还看得见开头
    let carets: Vec<usize> = boxed
        .hits
        .iter()
        .filter_map(|hit| match hit.target {
            HitTarget::Panel(PanelHit::Caret(i)) => Some(i),
            _ => None,
        })
        .collect();
    assert!(carets.contains(&0) && carets.contains(&4));
}

#[test]
fn a_selection_in_the_translate_box_keeps_its_click_targets() {
    let Ok(library) = FontLibrary::system("zh-CN") else {
        return;
    };
    let mut renderer = Renderer::new(library);
    let selected = PanelFrame {
        translate: Some(PanelTranslate {
            source: "今天天气很好".into(),
            cursor: 1,
            selection: Some(1..4),
            ..PanelTranslate::default()
        }),
        ..frame(5, 0, 10)
    };
    let rendered = renderer
        .render_panel(&selected, &Theme::light(), 2.0)
        .unwrap();
    let carets: Vec<usize> = rendered
        .hits
        .iter()
        .filter_map(|hit| match hit.target {
            HitTarget::Panel(PanelHit::Caret(i)) => Some(i),
            _ => None,
        })
        .collect();
    assert!((0..=6).all(|i| carets.contains(&i)));
}
