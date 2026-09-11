//! 候选窗口的绘制与测量：竖排一行一个候选，顶部一行拼音，一项高亮。GDI 自绘进分层窗口的位图（背景与阴影由 surface 铺）。
//!
//! 布局镜像 macOS 端 `candidates/view.rs` 的竖排：`序号  候选词   读音·词性 译文`，各列左对齐、
//! 小字底部对齐到候选词。横排 / 整句补全 / 云朵下一步补。所有坐标是「先定行（y）再定列（x）」。

use qingjian_platform::LayoutMode;
use qingjian_platform::protocol::PreeditKind;
use windows::Win32::Foundation::{COLORREF, RECT, SIZE};
use windows::Win32::Graphics::Gdi::{
    CreateRoundRectRgn, CreateSolidBrush, DeleteObject, FillRgn, GetTextExtentPoint32W, HDC, HFONT,
    SelectObject, SetBkMode, SetTextColor, TRANSPARENT, TextOutW,
};

use super::RenderData;
use super::row::{Row, Tone};
use super::theme::Theme;

/// 竖排的列宽与统一行高（像素）。
struct Columns {
    index_width: i32,
    text_width: i32,
    annotation_width: i32,
    row_height: i32,
}

/// 内容需要的窗口大小（像素，含内边距）。在真正建 / 移窗口前算。竖排 / 横排分开量。
pub(super) fn preferred_size(hdc: HDC, data: &RenderData) -> SIZE {
    let theme = &data.theme;
    let (top_width, top_height) = top_line_size(hdc, data);
    let (body_width, body_height) = match data.layout {
        LayoutMode::Vertical => vertical_size(hdc, data),
        LayoutMode::Horizontal => horizontal_size(hdc, data),
    };
    SIZE {
        cx: top_width.max(body_width) + theme.padding * 2,
        cy: top_height + body_height + theme.padding * 2,
    }
}

/// 竖排正文（不含顶部拼音行与内边距）的宽高。
fn vertical_size(hdc: HDC, data: &RenderData) -> (i32, i32) {
    let theme = &data.theme;
    let columns = columns(hdc, theme, &data.rows);
    let mut body_width = columns.index_width + theme.column_gap + columns.text_width;
    if columns.annotation_width > 0 {
        body_width += theme.column_gap + columns.annotation_width;
    }
    let mut body_height = columns.row_height * data.rows.len() as i32;
    if let Some(footer) = &data.footer {
        let size = measure(hdc, theme.index_font, footer);
        body_width = body_width.max(size.cx);
        body_height += size.cy + theme.row_padding;
    }
    (body_width, body_height)
}

/// 横排正文（不含顶部拼音行与内边距）的宽高：候选左右排 `序号 候选词`，高亮候选的译文另起一行在下方。
fn horizontal_size(hdc: HDC, data: &RenderData) -> (i32, i32) {
    let theme = &data.theme;
    if data.rows.is_empty() {
        return (0, 0);
    }
    let index_gap = theme.column_gap / 2;
    let highlight_inset = theme.padding / 2;
    let mut row_height = 0;
    let mut width = 0;
    for row in &data.rows {
        let index = measure(hdc, theme.index_font, &row.index);
        let text = measure(hdc, theme.text_font, &row.text);
        width += index.cx + index_gap + text.cx;
        row_height = row_height.max(text.cy + theme.row_padding * 2);
    }
    width += theme.column_gap * (data.rows.len().saturating_sub(1)) as i32 + highlight_inset * 2;
    if let Some(footer) = &data.footer {
        width += theme.column_gap + measure(hdc, theme.index_font, footer).cx;
    }
    let mut height = row_height;
    if let Some((annotation_width, annotation_height)) = highlighted_annotation_size(hdc, data) {
        width = width.max(annotation_width);
        height += annotation_height;
    }
    (width, height)
}

/// 横排时高亮候选的译文行尺寸；高亮候选没有译文（或没有高亮）时为 `None`。
fn highlighted_annotation_size(hdc: HDC, data: &RenderData) -> Option<(i32, i32)> {
    let theme = &data.theme;
    let row = data.rows.get(data.highlight)?;
    if row.annotation.is_empty() {
        return None;
    }
    let width: i32 = row
        .annotation
        .iter()
        .map(|(s, _)| measure(hdc, theme.annotation_font, s).cx)
        .sum();
    let height = line_height(hdc, theme.annotation_font) + theme.row_padding;
    Some((width, height))
}

/// 画整帧。`client` 是窗口客户区（已是最终大小）。竖排 / 横排分派。
pub(super) fn paint(hdc: HDC, data: &RenderData, client: RECT) {
    let theme = &data.theme;
    // SAFETY: hdc 是 BeginPaint 给的有效 DC。
    unsafe { SetBkMode(hdc, TRANSPARENT) };

    let mut y = theme.padding;
    y += draw_top_line(hdc, data, y);
    match data.layout {
        LayoutMode::Vertical => draw_rows(hdc, data, y, client.right - client.left),
        LayoutMode::Horizontal => draw_horizontal(hdc, data, y, client.right - client.left),
    }
}

/// 顶部拼音行：各段按样式画、自己画光标。返回占用高度（没有拼音时为 0）。
fn draw_top_line(hdc: HDC, data: &RenderData, y: i32) -> i32 {
    if data.preedit.is_empty() {
        return 0;
    }
    let theme = &data.theme;
    let height = line_height(hdc, theme.annotation_font);
    let top = y + theme.row_padding;
    let mut x = theme.padding;
    for (text, kind) in &data.preedit {
        let (color, strike) = match kind {
            PreeditKind::Typed => (theme.gloss_color, false),
            PreeditKind::Rest => (theme.pos_color, false),
            PreeditKind::Corrected => (theme.pos_color, true),
        };
        let width = draw_text(hdc, theme.annotation_font, color, x, top, text);
        if strike {
            // 纠错段画删除线：中线一道细横。
            let line = RECT {
                left: x,
                top: top + height / 2,
                right: x + width,
                bottom: top + height / 2 + scale_line(theme),
            };
            fill_rect(hdc, line, theme.pos_color);
        }
        x += width;
    }
    // 光标：拼音行前 cursor 个字符宽处画一道竖线。
    let before: String = concat_before_cursor(&data.preedit, data.cursor);
    let caret_x = theme.padding + measure(hdc, theme.annotation_font, &before).cx;
    let caret = RECT {
        left: caret_x,
        top,
        right: caret_x + scale_line(theme).max(1),
        bottom: top + height,
    };
    fill_rect(hdc, caret, theme.text_color);
    height + theme.row_padding * 2
}

/// 竖排候选行。
fn draw_rows(hdc: HDC, data: &RenderData, mut y: i32, width: i32) {
    let theme = &data.theme;
    let columns = columns(hdc, theme, &data.rows);
    let text_x = theme.padding + columns.index_width + theme.column_gap;
    let annotation_x = text_x + columns.text_width + theme.column_gap;
    for (i, row) in data.rows.iter().enumerate() {
        if i == data.highlight {
            let rect = RECT {
                left: theme.padding / 2,
                top: y,
                right: width - theme.padding / 2,
                bottom: y + columns.row_height,
            };
            fill_round_rect(hdc, rect, theme.highlight, theme.corner_radius / 2);
        }
        let baseline = y + theme.row_padding;
        let text_size = measure(hdc, theme.text_font, &row.text);
        let small_offset = small_offset(hdc, theme, text_size.cy);
        draw_text(
            hdc,
            theme.index_font,
            theme.index_color,
            theme.padding,
            baseline + small_offset,
            &row.index,
        );
        let color = if row.cloud {
            theme.cloud_color
        } else {
            theme.text_color
        };
        draw_text(hdc, theme.text_font, color, text_x, baseline, &row.text);
        let mut x = annotation_x;
        for (segment, tone) in &row.annotation {
            x += draw_text(
                hdc,
                theme.annotation_font,
                tone_color(theme, *tone),
                x,
                baseline + small_offset,
                segment,
            );
        }
        y += columns.row_height;
    }
    if let Some(footer) = &data.footer {
        let size = measure(hdc, theme.index_font, footer);
        draw_text(
            hdc,
            theme.index_font,
            theme.index_color,
            width - theme.padding - size.cx,
            y + theme.row_padding,
            footer,
        );
    }
}

/// 横排候选行：候选左右排 `序号 候选词`，高亮项加圆角底、译文另起一行在下方，页码在首行右端。
fn draw_horizontal(hdc: HDC, data: &RenderData, y: i32, width: i32) {
    if data.rows.is_empty() {
        return;
    }
    let theme = &data.theme;
    let index_gap = theme.column_gap / 2;
    let highlight_inset = theme.padding / 2;
    let row_height = data
        .rows
        .iter()
        .map(|row| measure(hdc, theme.text_font, &row.text).cy + theme.row_padding * 2)
        .max()
        .unwrap_or(0);
    let baseline = y + theme.row_padding;
    let mut x = theme.padding + highlight_inset;
    for (i, row) in data.rows.iter().enumerate() {
        let index_width = measure(hdc, theme.index_font, &row.index).cx;
        let text_size = measure(hdc, theme.text_font, &row.text);
        let item_width = index_width + index_gap + text_size.cx;
        if i == data.highlight {
            let rect = RECT {
                left: x - highlight_inset,
                top: y,
                right: x - highlight_inset + item_width + highlight_inset * 2,
                bottom: y + row_height,
            };
            fill_round_rect(hdc, rect, theme.highlight, theme.corner_radius / 2);
        }
        let small_offset = small_offset(hdc, theme, text_size.cy);
        draw_text(
            hdc,
            theme.index_font,
            theme.index_color,
            x,
            baseline + small_offset,
            &row.index,
        );
        let color = if row.cloud {
            theme.cloud_color
        } else {
            theme.text_color
        };
        draw_text(
            hdc,
            theme.text_font,
            color,
            x + index_width + index_gap,
            baseline,
            &row.text,
        );
        x += item_width + theme.column_gap;
    }
    if let Some(footer) = &data.footer {
        let size = measure(hdc, theme.index_font, footer);
        let offset = small_offset(hdc, theme, line_height(hdc, theme.text_font));
        draw_text(
            hdc,
            theme.index_font,
            theme.index_color,
            width - theme.padding - size.cx,
            baseline + offset,
            footer,
        );
    }
    // 高亮候选的译文，另起一行在候选行下方。
    if let Some(row) = data.rows.get(data.highlight) {
        let mut x = theme.padding + highlight_inset;
        let top = y + row_height + theme.row_padding / 2;
        for (segment, tone) in &row.annotation {
            x += draw_text(
                hdc,
                theme.annotation_font,
                tone_color(theme, *tone),
                x,
                top,
                segment,
            );
        }
    }
}

/// 顶部拼音行需要的宽高；没有拼音时都是 0。
fn top_line_size(hdc: HDC, data: &RenderData) -> (i32, i32) {
    if data.preedit.is_empty() {
        return (0, 0);
    }
    let theme = &data.theme;
    let height = line_height(hdc, theme.annotation_font);
    let full: String = data.preedit.iter().map(|(t, _)| t.as_str()).collect();
    let width = measure(hdc, theme.annotation_font, &full).cx + scale_line(theme).max(1);
    (width, height + theme.row_padding * 2)
}

/// 竖排各列宽与统一行高。
fn columns(hdc: HDC, theme: &Theme, rows: &[Row]) -> Columns {
    let mut columns = Columns {
        index_width: 0,
        text_width: 0,
        annotation_width: 0,
        row_height: 0,
    };
    for row in rows {
        let index = measure(hdc, theme.index_font, &row.index);
        let text = measure(hdc, theme.text_font, &row.text);
        let annotation: i32 = row
            .annotation
            .iter()
            .map(|(s, _)| measure(hdc, theme.annotation_font, s).cx)
            .sum();
        columns.index_width = columns.index_width.max(index.cx);
        columns.text_width = columns.text_width.max(text.cx);
        columns.annotation_width = columns.annotation_width.max(annotation);
        columns.row_height = columns.row_height.max(text.cy + theme.row_padding * 2);
    }
    columns
}

/// 小字（序号 / 读音 / 词性 / 译文）相对候选词往下挪多少，让两者**纵向居中**对齐。
///
/// macOS 端用「文字高 − 小字高」的全差，但那边坐标系 y 向上、视觉上是居中；GDI 这边 y 向下、全差会变成
/// 底对齐，所以取一半，效果才和 macOS 一致。
fn small_offset(hdc: HDC, theme: &Theme, text_height: i32) -> i32 {
    ((text_height - line_height(hdc, theme.annotation_font)) / 2).max(0)
}

fn tone_color(theme: &Theme, tone: Tone) -> COLORREF {
    match tone {
        Tone::Gloss => theme.gloss_color,
        Tone::Fresh => theme.fresh_color,
        Tone::Faint => theme.pos_color,
    }
}

/// 拼音行光标前 `cursor` 个字符拼成的串（按各段顺序）。
fn concat_before_cursor(preedit: &[(String, PreeditKind)], cursor: usize) -> String {
    let mut out = String::new();
    let mut remaining = cursor;
    for (text, _) in preedit {
        for ch in text.chars() {
            if remaining == 0 {
                return out;
            }
            out.push(ch);
            remaining -= 1;
        }
    }
    out
}

/// 画一段文字，返回宽度。`(x, y)` 是左上角。
fn draw_text(hdc: HDC, font: HFONT, color: COLORREF, x: i32, y: i32, text: &str) -> i32 {
    let utf16: Vec<u16> = text.encode_utf16().collect();
    // SAFETY: hdc 有效；font 属于本主题；utf16 是有效切片。
    unsafe {
        SelectObject(hdc, font.into());
        SetTextColor(hdc, color);
        let _ = TextOutW(hdc, x, y, &utf16);
    }
    measure(hdc, font, text).cx
}

/// 量一段文字的宽高（选中 `font` 后调 GetTextExtentPoint32W）。
fn measure(hdc: HDC, font: HFONT, text: &str) -> SIZE {
    let utf16: Vec<u16> = text.encode_utf16().collect();
    let mut size = SIZE::default();
    // SAFETY: hdc 有效；font 属于本主题。
    unsafe {
        SelectObject(hdc, font.into());
        let _ = GetTextExtentPoint32W(hdc, &utf16, &mut size);
    }
    size
}

/// 一行文字的高度（用参考字 `x` 量）。
fn line_height(hdc: HDC, font: HFONT) -> i32 {
    measure(hdc, font, "x").cy
}

/// 细线 / 光标的粗细，随 DPI 放大（至少 1px）。
fn scale_line(theme: &Theme) -> i32 {
    (theme.padding / 8).max(1)
}

/// 用纯色刷子填一个矩形。
fn fill_rect(hdc: HDC, rect: RECT, color: COLORREF) {
    // SAFETY: hdc 有效；brush 造出后即用即删。
    unsafe {
        let brush = CreateSolidBrush(color);
        windows::Win32::Graphics::Gdi::FillRect(hdc, &rect, brush);
        let _ = DeleteObject(brush.into());
    }
}

/// 用纯色填一个圆角矩形（候选高亮用）。
fn fill_round_rect(hdc: HDC, rect: RECT, color: COLORREF, radius: i32) {
    let diameter = (radius * 2).max(1);
    // SAFETY: 区域与刷子造出后即用即删。
    unsafe {
        let region = CreateRoundRectRgn(
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            diameter,
            diameter,
        );
        let brush = CreateSolidBrush(color);
        let _ = FillRgn(hdc, region, brush);
        let _ = DeleteObject(brush.into());
        let _ = DeleteObject(region.into());
    }
}
