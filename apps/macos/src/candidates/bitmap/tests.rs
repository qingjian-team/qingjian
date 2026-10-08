//! 用本机真实字体核对字族选择、独立加粗、字号布局与缺字回退。

use qingjian_render::{FontLibrary, FontRole, Frame, Layout, Renderer, Row, Theme, Tone, UiFont};

use super::font_files::family_files;

fn renderer(candidate: &str, annotation: &str) -> Renderer {
    let font = |family: &str| UiFont {
        family: family.into(),
        files: family_files(family),
    };
    Renderer::new(
        FontLibrary::with_candidate_fonts("zh-CN", &font(candidate), &font(annotation)).unwrap(),
    )
}

fn theme() -> Theme {
    let mut theme = Theme::light();
    theme.text_font.role = FontRole::Candidate;
    theme.annotation_font.role = FontRole::Annotation;
    theme
}

#[test]
fn chinese_and_translation_resolve_to_different_selected_families() {
    let mut renderer = renderer("Songti SC", "Times New Roman");
    let mut theme = theme();
    assert_eq!(renderer.trace_families("中国你好", &theme), ["Songti SC"]);
    theme.text_font.role = FontRole::Annotation;
    assert_eq!(
        renderer.trace_families("hello language", &theme),
        ["Times New Roman"]
    );
}

#[test]
fn absent_family_and_missing_chinese_glyphs_fall_back() {
    let mut renderer = renderer("Qingjian nonexistent family", "Times New Roman");
    let mut theme = theme();
    assert!(
        renderer
            .trace_families("中国", &theme)
            .iter()
            .any(|name| name.contains("PingFang"))
    );
    theme.text_font.role = FontRole::Annotation;
    assert!(
        renderer
            .trace_families("中国", &theme)
            .iter()
            .any(|name| name.contains("PingFang"))
    );
}

fn pixels(renderer: &mut Renderer, frame: &Frame, theme: &Theme) -> Vec<u8> {
    renderer
        .render(frame, Layout::Vertical, theme, 2.0, None)
        .unwrap()
        .pixmap
        .data()
        .to_vec()
}

#[test]
fn bold_changes_only_the_requested_role_including_single_weight_fonts() {
    // BM Dohyeon 在 macOS 可选字体中只有常规面，需合成粗体。
    if family_files("BM Dohyeon").is_empty() {
        return;
    }
    let mut renderer = renderer("Songti SC", "BM Dohyeon");
    let candidate_only = Frame {
        rows: vec![Row::plain(0, "中国你好")],
        ..Frame::default()
    };
    let annotation_only = Frame {
        rows: vec![Row {
            index: String::new(),
            text: String::new(),
            foreign_text: false,
            chinese_annotation: false,
            annotation: vec![("language".into(), Tone::Gloss)],
            code: None,
            cloud: false,
        }],
        ..Frame::default()
    };
    let mut theme = theme();
    let mut family_probe = theme.clone();
    family_probe.text_font.role = FontRole::Annotation;
    let regular_family = renderer.trace_families("language", &family_probe);
    family_probe.text_font.bold = true;
    assert_eq!(
        renderer.trace_families("language", &family_probe),
        regular_family
    );
    let cn_regular = pixels(&mut renderer, &candidate_only, &theme);
    let en_regular = pixels(&mut renderer, &annotation_only, &theme);
    theme.annotation_font.bold = true;
    assert_eq!(cn_regular, pixels(&mut renderer, &candidate_only, &theme));
    assert_ne!(en_regular, pixels(&mut renderer, &annotation_only, &theme));
    theme.annotation_font.bold = false;
    theme.text_font.bold = true;
    assert_ne!(cn_regular, pixels(&mut renderer, &candidate_only, &theme));
    // 空候选不应凭空产生合成粗体的宽度。
    assert_eq!(en_regular, pixels(&mut renderer, &annotation_only, &theme));
}

#[test]
fn taller_translation_expands_each_vertical_row() {
    let mut renderer = renderer("Songti SC", "Times New Roman");
    let mut row = Row::plain(0, "学习");
    row.annotation = vec![("language".into(), Tone::Gloss)];
    let frame = Frame {
        rows: vec![row.clone(), row],
        ..Frame::default()
    };
    let mut theme = theme();
    theme.text_font.size = 8.0;
    theme.text_font.line_height = 11.0;
    theme.annotation_font.size = 48.0;
    theme.annotation_font.line_height = 63.0;
    let rendered = renderer
        .render(&frame, Layout::Vertical, &theme, 2.0, None)
        .unwrap();
    assert_eq!(rendered.content_size_points().1, 16.0 + 2.0 * (63.0 + 8.0));
}

#[test]
fn english_rows_match_explicitly_swapped_font_profiles_in_every_layout() {
    let mut renderer = renderer("Songti SC", "Times New Roman");
    let mut theme = theme();
    theme.text_font.size = 28.0;
    theme.text_font.line_height = 37.0;
    theme.text_font.bold = true;
    theme.annotation_font.size = 18.0;
    theme.annotation_font.line_height = 24.0;
    theme.pos_font.role = FontRole::Annotation;
    let mut row = Row::plain(0, "learning");
    row.foreign_text = true;
    row.chinese_annotation = true;
    row.annotation = vec![
        ("v. ".into(), Tone::PartOfSpeech),
        ("学习".into(), Tone::Gloss),
    ];
    for (layout, columns) in [
        (Layout::Vertical, 0),
        (Layout::Horizontal, 0),
        (Layout::Horizontal, 1),
    ] {
        let mut frame = Frame {
            rows: vec![row.clone()],
            highlighted: Some(0),
            columns,
            column_ems: if columns > 0 { vec![2.0] } else { vec![] },
            ..Frame::default()
        };
        let actual = renderer.render(&frame, layout, &theme, 2.0, None).unwrap();
        frame.rows[0].foreign_text = false;
        frame.rows[0].chinese_annotation = false;
        let mut swapped = theme.clone();
        std::mem::swap(&mut swapped.text_font, &mut swapped.annotation_font);
        let expected = renderer
            .render(&frame, layout, &swapped, 2.0, None)
            .unwrap();
        assert_eq!(actual.pixmap, expected.pixmap, "{layout:?}/{columns}");
    }
}

#[test]
fn large_pos_expands_annotation_rows_in_every_layout() {
    let mut renderer = renderer("Songti SC", "Times New Roman");
    let mut row = Row::plain(0, "学习");
    row.annotation = vec![
        ("v. ".into(), Tone::PartOfSpeech),
        ("learn".into(), Tone::Gloss),
    ];
    let baseline = theme();
    let mut enlarged = baseline.clone();
    enlarged.pos_font.size = 48.0;
    enlarged.pos_font.line_height = 63.0;
    for (layout, columns) in [
        (Layout::Vertical, 0),
        (Layout::Horizontal, 0),
        (Layout::Horizontal, 1),
    ] {
        let frame = Frame {
            rows: vec![row.clone()],
            highlighted: Some(0),
            columns,
            ..Frame::default()
        };
        let small = renderer
            .render(&frame, layout, &baseline, 2.0, None)
            .unwrap();
        let large = renderer
            .render(&frame, layout, &enlarged, 2.0, None)
            .unwrap();
        assert!(
            large.content_height > small.content_height,
            "{layout:?}/{columns}"
        );
        assert_ne!(small.pixmap, large.pixmap);
    }
}

#[test]
fn emoji_chinese_labels_match_chinese_font_size_and_weight_in_all_layouts() {
    use qingjian_core::{Candidate, CandidateKind};
    let candidate = Candidate {
        text: "🔶".into(),
        kind: CandidateKind::Emoji,
        syllables: vec![],
        reading: Some("大".into()),
        translation: None,
        aux_code: None,
    };
    // 连同平台帧的转换一起验证，避免只在某一条绘制路径修好。
    let frame = crate::candidates::Frame {
        rows: vec![crate::candidates::Row::from_candidate(0, &candidate)],
        ..crate::candidates::Frame::default()
    };
    let mut frame = super::convert::frame(&frame);
    assert!(!frame.rows[0].foreign_text);
    assert_eq!(
        frame.rows[0].annotation[0].1,
        Tone::Reading { chinese: true }
    );
    let mut renderer = renderer("Songti SC", "Times New Roman");
    let mut theme = theme();
    theme.text_font.size = 28.0;
    theme.text_font.line_height = 37.0;
    theme.text_font.bold = true;
    for (layout, columns) in [
        (Layout::Vertical, 0),
        (Layout::Horizontal, 0),
        (Layout::Horizontal, 1),
    ] {
        frame.columns = columns;
        let actual = renderer.render(&frame, layout, &theme, 2.0, None).unwrap();
        let mut expected_frame = frame.clone();
        expected_frame.rows[0].annotation[0].1 = Tone::Reading { chinese: false };
        let mut explicit_chinese = theme.clone();
        explicit_chinese.annotation_font = theme.text_font;
        let expected = renderer
            .render(&expected_frame, layout, &explicit_chinese, 2.0, None)
            .unwrap();
        assert_eq!(actual.pixmap, expected.pixmap, "{layout:?}/{columns}");
        let old_style = renderer
            .render(&expected_frame, layout, &theme, 2.0, None)
            .unwrap();
        assert_ne!(actual.pixmap, old_style.pixmap, "未复现原来的字体回退差异");
    }
}
