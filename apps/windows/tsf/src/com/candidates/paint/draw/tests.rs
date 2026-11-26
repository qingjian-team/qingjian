//! 量尺寸与出图的冒烟测试。

use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};

use qingjian_core::{Candidate, CandidateKind, CandidateList};
use qingjian_platform::LayoutMode;
use qingjian_platform::protocol::Frame;

use super::preferred_size;
use crate::com::candidates::paint::theme::Theme;
use crate::com::candidates::paint::view::View;

fn frame(layout: LayoutMode, columns: usize) -> Frame {
    Frame {
        candidates: CandidateList {
            items: ["你", "泥", "拟", "逆"]
                .into_iter()
                .map(|text| Candidate {
                    text: text.to_owned(),
                    kind: CandidateKind::Chinese,
                    syllables: Vec::new(),
                    reading: None,
                    translation: None,
                    aux_code: None,
                })
                .collect(),
        },
        layout,
        columns,
        highlight: 0,
        page_count: 1,
        ..Frame::default()
    }
}

/// 三种排布量出来都是正尺寸，横排比竖排宽、矩阵比横排高（各排布真的走了不同的分支）。
#[test]
fn layouts_measure_differently() {
    let theme = Theme::new(96, false, 16, 12, None);
    let hdc = unsafe { GetDC(None) };
    let mut view = View::default();

    view.set(&frame(LayoutMode::Vertical, 0));
    let vertical = preferred_size(hdc, &theme, &view);

    view.set(&frame(LayoutMode::Horizontal, 0));
    let horizontal = preferred_size(hdc, &theme, &view);

    view.set(&frame(LayoutMode::Horizontal, 2));
    let matrix = preferred_size(hdc, &theme, &view);
    unsafe { ReleaseDC(None, hdc) };

    for size in [vertical, horizontal, matrix] {
        assert!(size.cx > 0 && size.cy > 0, "{size:?}");
    }
    assert!(horizontal.cx > vertical.cx, "横排应当比竖排宽");
    assert!(matrix.cy > horizontal.cy, "两行矩阵应当比单行高");
}

/// 把一帧画到位图上、存成 PNG：不截屏，直接看画布画成了什么样。人工看一眼用。
#[test]
#[ignore = "写一张 PNG 供人工查看"]
fn a_frame_saves_a_png() {
    use qingjian_core::{Candidate, CandidateKind, CandidateList};
    use qingjian_platform::protocol::{Frame, PreeditKind, PreeditSegment};
    use windows::Win32::Foundation::RECT;

    use super::paint;
    use crate::com::candidates::paint::Canvas;

    let theme = Theme::new(96, false, 16, 12, None);
    let frame = Frame {
        preedit: vec![PreeditSegment {
            text: "ni'hao".to_owned(),
            kind: PreeditKind::Typed,
        }],
        cursor: 6,
        candidates: CandidateList {
            items: vec!["你好", "拟好", "你号"]
                .into_iter()
                .map(|text| Candidate {
                    text: text.to_owned(),
                    kind: CandidateKind::Chinese,
                    syllables: Vec::new(),
                    reading: None,
                    translation: None,
                    aux_code: None,
                })
                .collect(),
        },
        highlight: 0,
        page_count: 2,
        ..Frame::default()
    };
    let mut view = View::default();
    view.set(&frame);
    let screen = unsafe { GetDC(None) };
    let size = preferred_size(screen, &theme, &view);
    unsafe { ReleaseDC(None, screen) };
    let mut canvas = Canvas::new(size.cx, size.cy).expect("画布");
    canvas.fill_background(theme.background, theme.border, theme.corner_radius);
    let hits = paint(
        canvas.dc(),
        &theme,
        &view,
        RECT {
            left: 0,
            top: 0,
            right: size.cx,
            bottom: size.cy,
        },
    );
    canvas.finish(theme.corner_radius);
    let (width, height) = canvas.size();
    println!("canvas {width}x{height} hits={}", hits.len());
    let file = std::fs::File::create(std::env::temp_dir().join("qingjian-candidate-canvas.png"))
        .expect("png");
    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(file),
        u32::try_from(width).unwrap(),
        u32::try_from(height).unwrap(),
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut writer| {
            writer.write_image_data(
                &canvas
                    .pixels()
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .flat_map(|px| [px[2], px[1], px[0], px[3]])
                    .collect::<Vec<u8>>(),
            )
        })
        .expect("写 PNG");
}

/// 空帧量出内边距那一圈；没有内容也应当是正尺寸。
#[test]
fn an_empty_view_still_has_padding() {
    let theme = Theme::new(96, false, 16, 12, None);
    let hdc = unsafe { GetDC(None) };
    let size = preferred_size(hdc, &theme, &View::default());
    unsafe { ReleaseDC(None, hdc) };
    assert_eq!(size.cx, theme.padding * 2);
    assert_eq!(size.cy, theme.padding * 2);
}
