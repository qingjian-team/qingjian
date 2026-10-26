//! 轮廓编号图：每个待挑的部件把整字画大，整字先按非零规则填成浅灰（空心照样空着），每个轮廓再描一圈彩色边、在起点标编号；
//! 底下每 100 单位一条网格线（0–1000 的字面方块），定裁剪框时照着读坐标。数字也取自同一个字体。

use std::path::Path;

use skrifa::FontRef;
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform};

use super::outline::{Outline, Segment};

/// 每格边长（像素），字画在里面留出边距与标题。
const CELL: u32 = 320;

/// 一行几格。
const COLUMNS: u32 = 5;

/// 字在格子里的大小（像素，对应 1000 单位）。
const GLYPH: f32 = 240.0;

/// 编号数字的大小（像素）。
const DIGIT: f32 = 34.0;

/// 相邻笔画用对比明显的颜色区分。
const PALETTE: [(u8, u8, u8); 8] = [
    (220, 50, 47),
    (38, 139, 210),
    (133, 153, 0),
    (211, 54, 130),
    (181, 137, 0),
    (42, 161, 152),
    (108, 113, 196),
    (203, 75, 22),
];

/// `items` 是（说明，整字的轮廓），格子左上角标序号，说明由调用方按序号打印出来对照。
pub fn write(
    font: &FontRef,
    items: &[(String, Outline)],
    path: &Path,
) -> Result<(), std::io::Error> {
    let rows = (items.len() as u32).div_ceil(COLUMNS).max(1);
    let mut pixmap = Pixmap::new(CELL * COLUMNS, CELL * rows)
        .ok_or_else(|| std::io::Error::other("sheet too large"))?;
    pixmap.fill(Color::WHITE);
    for (index, (_, outline)) in items.iter().enumerate() {
        let (column, row) = (index as u32 % COLUMNS, index as u32 / COLUMNS);
        let (left, top) = ((column * CELL) as f32 + 40.0, (row * CELL) as f32 + 40.0);
        let scale = GLYPH / 1000.0;
        let transform = Transform::from_scale(scale, scale).post_translate(left, top);
        // 网格：每 100 单位一条，500 加深
        for step in 0..=10 {
            let at = step as f32 * 100.0;
            let shade = if step % 5 == 0 { 170 } else { 215 };
            let mut grid = PathBuilder::new();
            grid.move_to(at, 0.0);
            grid.line_to(at, 1000.0);
            grid.move_to(0.0, at);
            grid.line_to(1000.0, at);
            if let Some(grid) = grid.finish() {
                let stroke = Stroke {
                    width: 1.0 / scale,
                    ..Stroke::default()
                };
                pixmap.stroke_path(
                    &grid,
                    &solid((shade, shade, shade), 255),
                    &stroke,
                    transform,
                    None,
                );
            }
        }
        if let Some(whole) = glyph_path(&outline.contours) {
            pixmap.fill_path(
                &whole,
                &solid((150, 150, 150), 110),
                FillRule::Winding,
                transform,
                None,
            );
        }
        for (i, contour) in outline.contours.iter().enumerate() {
            let color = PALETTE[i % PALETTE.len()];
            if let Some(path) = glyph_path(std::slice::from_ref(contour)) {
                let stroke = Stroke {
                    width: 2.5 / scale,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &solid(color, 255), &stroke, transform, None);
            }
            let (x, y) = outline.start(i);
            draw_number(
                font,
                &mut pixmap,
                i,
                (left + x * scale, top + y * scale),
                color,
            );
        }
        // 格子编号，与终端里打印的清单对照
        draw_number(
            font,
            &mut pixmap,
            index,
            ((column * CELL) as f32 + 6.0, (row * CELL) as f32 + 4.0),
            (0, 0, 0),
        );
    }
    pixmap
        .save_png(path)
        .map_err(|error| std::io::Error::other(error.to_string()))
}

fn solid((r, g, b): (u8, u8, u8), alpha: u8) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(r, g, b, alpha);
    paint.anti_alias = true;
    paint
}

/// 几个轮廓合成一条路径（每个轮廓各自闭合）。
fn glyph_path(contours: &[Vec<Segment>]) -> Option<tiny_skia::Path> {
    let mut builder = PathBuilder::new();
    for contour in contours {
        for segment in contour {
            match *segment {
                Segment::Move(x, y) => builder.move_to(x, y),
                Segment::Line(x, y) => builder.line_to(x, y),
                Segment::Quad(a, b, x, y) => builder.quad_to(a, b, x, y),
                Segment::Cubic(a, b, c, d, x, y) => builder.cubic_to(a, b, c, d, x, y),
            }
        }
        builder.close();
    }
    builder.finish()
}

/// 在 `(x, y)`（左上角）画一个十进制数，白底衬着免得压在笔画上看不清。
fn draw_number(
    font: &FontRef,
    pixmap: &mut Pixmap,
    number: usize,
    (x, y): (f32, f32),
    color: (u8, u8, u8),
) {
    let text = number.to_string();
    let scale = DIGIT / 1000.0;
    let width = text.len() as f32 * DIGIT * 0.6;
    if let Some(back) = tiny_skia::Rect::from_xywh(x - 2.0, y - 2.0, width + 4.0, DIGIT * 0.95) {
        pixmap.fill_rect(
            back,
            &solid((255, 255, 255), 230),
            Transform::identity(),
            None,
        );
    }
    for (i, ch) in text.chars().enumerate() {
        let Some(digit) = Outline::load(font, ch) else {
            continue;
        };
        {
            if let Some(path) = glyph_path(&digit.contours) {
                // 数字的字面也按 0–1000 排，往上挪一点贴齐方块顶
                let transform = Transform::from_scale(scale, scale)
                    .post_translate(x + i as f32 * DIGIT * 0.6 - DIGIT * 0.2, y - DIGIT * 0.12);
                pixmap.fill_path(
                    &path,
                    &solid(color, 255),
                    FillRule::Winding,
                    transform,
                    None,
                );
            }
        }
    }
}
