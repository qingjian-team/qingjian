//! 候选窗口的绘制与测量。布局镜像 macOS 端 `candidates/view.rs`：竖排 `序号  候选词   读音·词性 译文`，
//! 横排 `序号 候选词` 左右排、高亮项的译文另起一行；顶部一行拼音，右侧整句补全。

use qingjian_platform::LayoutMode;
use qingjian_platform::protocol::PreeditKind;
use windows::Win32::Foundation::{COLORREF, RECT, SIZE};
use windows::Win32::Graphics::Gdi::{
    CreateRoundRectRgn, CreateSolidBrush, DeleteObject, FillRect, FillRgn, GetTextExtentPoint32W,
    HDC, HFONT, SelectObject, SetBkMode, SetTextColor, TRANSPARENT, TextOutW,
};

use super::RenderData;
use super::row::{Row, Tone};
use super::theme::Theme;

/// 云端候选词前的小云朵（macOS 用 SF Symbol `cloud`，这边用字形）。
const CLOUD_GLYPH: &str = "☁";

/// 竖排的列宽与统一行高。
struct Columns {
    index_width: i32,
    text_width: i32,
    annotation_width: i32,
    row_height: i32,
}

/// 内容需要的大小（含内边距）。
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
        width += index.cx + index_gap + cloud_prefix_width(hdc, theme, row) + text.cx;
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

/// 横排时高亮候选的译文行尺寸；没有译文 / 没有高亮时为 `None`。
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

/// 画整帧。背景与阴影已由 surface 铺好，`client` 是内容区。
pub(super) fn paint(hdc: HDC, data: &RenderData, client: RECT) {
    let theme = &data.theme;
    // SAFETY: hdc 有效。
    unsafe { SetBkMode(hdc, TRANSPARENT) };

    let mut y = theme.padding;
    y += draw_top_line(hdc, data, y);
    match data.layout {
        LayoutMode::Vertical => draw_rows(hdc, data, y, client.right - client.left),
        LayoutMode::Horizontal => draw_horizontal(hdc, data, y, client.right - client.left),
    }
}

/// 顶部拼音行：各段按样式画、自己画光标、右侧整句补全。返回占用高度。
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
    let before = concat_before_cursor(&data.preedit, data.cursor);
    let caret_x = theme.padding + measure(hdc, theme.annotation_font, &before).cx;
    let caret = RECT {
        left: caret_x,
        top,
        right: caret_x + scale_line(theme),
        bottom: top + height,
    };
    fill_rect(hdc, caret, theme.text_color);
    if let Some(sentence) = &data.sentence {
        draw_text(
            hdc,
            theme.annotation_font,
            theme.gloss_color,
            x + theme.column_gap,
            top,
            sentence,
        );
    }
    height + theme.row_padding * 2
}

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
        draw_word(hdc, theme, row, text_x, baseline, small_offset);
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
        let item_width =
            index_width + index_gap + cloud_prefix_width(hdc, theme, row) + text_size.cx;
        if i == data.highlight {
            let rect = RECT {
                left: x - highlight_inset,
                top: y,
                right: x + item_width + highlight_inset,
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
        draw_word(
            hdc,
            theme,
            row,
            x + index_width + index_gap,
            baseline,
            small_offset,
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

fn top_line_size(hdc: HDC, data: &RenderData) -> (i32, i32) {
    if data.preedit.is_empty() {
        return (0, 0);
    }
    let theme = &data.theme;
    let height = line_height(hdc, theme.annotation_font);
    let full: String = data.preedit.iter().map(|(t, _)| t.as_str()).collect();
    let mut width = measure(hdc, theme.annotation_font, &full).cx + scale_line(theme);
    if let Some(sentence) = &data.sentence {
        width += theme.column_gap + measure(hdc, theme.annotation_font, sentence).cx;
    }
    (width, height + theme.row_padding * 2)
}

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
        columns.text_width = columns
            .text_width
            .max(text.cx + cloud_prefix_width(hdc, theme, row));
        columns.annotation_width = columns.annotation_width.max(annotation);
        columns.row_height = columns.row_height.max(text.cy + theme.row_padding * 2);
    }
    columns
}

/// 小字相对候选词往下挪多少才纵向居中。macOS 用全差是因为那边 y 向上；GDI y 向下要取一半。
fn small_offset(hdc: HDC, theme: &Theme, text_height: i32) -> i32 {
    ((text_height - line_height(hdc, theme.annotation_font)) / 2).max(0)
}

/// 云朵前缀（云朵 + 间距）的宽度；非云端候选为 0。
fn cloud_prefix_width(hdc: HDC, theme: &Theme, row: &Row) -> i32 {
    if row.cloud {
        measure(hdc, theme.annotation_font, CLOUD_GLYPH).cx + theme.column_gap / 2
    } else {
        0
    }
}

/// 画候选词本体，云端词前带小云朵（云朵与词都用 teal）。返回占用宽度。
fn draw_word(hdc: HDC, theme: &Theme, row: &Row, x: i32, baseline: i32, small_offset: i32) -> i32 {
    let prefix = cloud_prefix_width(hdc, theme, row);
    let color = if row.cloud {
        draw_text(
            hdc,
            theme.annotation_font,
            theme.cloud_color,
            x,
            baseline + small_offset,
            CLOUD_GLYPH,
        );
        theme.cloud_color
    } else {
        theme.text_color
    };
    prefix + draw_text(hdc, theme.text_font, color, x + prefix, baseline, &row.text)
}

fn tone_color(theme: &Theme, tone: Tone) -> COLORREF {
    match tone {
        Tone::Gloss => theme.gloss_color,
        Tone::Fresh => theme.fresh_color,
        Tone::Faint => theme.pos_color,
    }
}

/// 拼音行光标前 `cursor` 个字符。
fn concat_before_cursor(preedit: &[(String, PreeditKind)], cursor: usize) -> String {
    preedit
        .iter()
        .flat_map(|(text, _)| text.chars())
        .take(cursor)
        .collect()
}

/// 画一段文字，`(x, y)` 是左上角，返回宽度。
fn draw_text(hdc: HDC, font: HFONT, color: COLORREF, x: i32, y: i32, text: &str) -> i32 {
    let utf16: Vec<u16> = text.encode_utf16().collect();
    let mut size = SIZE::default();
    // SAFETY: hdc 有效；font 属于当前主题；utf16 是有效切片。
    unsafe {
        SelectObject(hdc, font.into());
        SetTextColor(hdc, color);
        let _ = TextOutW(hdc, x, y, &utf16);
        let _ = GetTextExtentPoint32W(hdc, &utf16, &mut size);
    }
    size.cx
}

fn measure(hdc: HDC, font: HFONT, text: &str) -> SIZE {
    let utf16: Vec<u16> = text.encode_utf16().collect();
    let mut size = SIZE::default();
    // SAFETY: hdc 有效；font 属于当前主题。
    unsafe {
        SelectObject(hdc, font.into());
        let _ = GetTextExtentPoint32W(hdc, &utf16, &mut size);
    }
    size
}

fn line_height(hdc: HDC, font: HFONT) -> i32 {
    measure(hdc, font, "x").cy
}

/// 细线 / 光标的粗细，随 DPI 放大。
fn scale_line(theme: &Theme) -> i32 {
    (theme.padding / 8).max(1)
}

fn fill_rect(hdc: HDC, rect: RECT, color: COLORREF) {
    // SAFETY: hdc 有效；刷子即用即删。
    unsafe {
        let brush = CreateSolidBrush(color);
        FillRect(hdc, &rect, brush);
        let _ = DeleteObject(brush.into());
    }
}

fn fill_round_rect(hdc: HDC, rect: RECT, color: COLORREF, radius: i32) {
    let diameter = (radius * 2).max(1);
    // SAFETY: 区域与刷子即用即删。
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
