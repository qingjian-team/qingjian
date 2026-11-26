//! 画一帧用到的小工具：量字、截尾、落笔、填色，以及几处由主题与行数据推出来的宽度。

use windows::Win32::Foundation::{COLORREF, RECT, SIZE};
use windows::Win32::Graphics::Gdi::{
    CreateRoundRectRgn, CreateSolidBrush, DeleteObject, FillRect, FillRgn, GetTextExtentPoint32W,
    HDC, HFONT, SelectObject, SetTextColor, TextOutW,
};

use qingjian_platform::protocol::PreeditKind;

use super::{CLOUD_GLYPH, Columns};
use crate::com::candidates::paint::row::{Row, Tone};
use crate::com::candidates::paint::theme::Theme;

/// 超宽候选截尾补的省略号。
const ELLIPSIS: &str = "…";

/// 信息行里画一段小字，宽度超过 `budget` 就截断；返回画了多宽。
pub(super) fn draw_clipped(
    hdc: HDC,
    theme: &Theme,
    text: &str,
    color: COLORREF,
    x: i32,
    y: i32,
    budget: i32,
) -> i32 {
    let (shown, _) = truncate(hdc, theme.annotation_font, text, budget.max(0) as f32);
    draw_text(hdc, theme.annotation_font, color, x, y, &shown)
}

/// `text` 宽度超过 `max_width` 就从末尾去字、补上「…」直到放得下；返回显示文字与是否截断过。
pub(super) fn truncate(hdc: HDC, font: HFONT, text: &str, max_width: f32) -> (String, bool) {
    if text.is_empty() || measure(hdc, font, text).cx as f32 <= max_width {
        return (text.to_owned(), false);
    }
    let kept: Vec<char> = text.chars().collect();
    for n in (0..kept.len()).rev() {
        let mut candidate: String = kept[..n].iter().collect();
        candidate.push_str(ELLIPSIS);
        if n == 0 || measure(hdc, font, &candidate).cx as f32 <= max_width {
            return (candidate, true);
        }
    }
    (ELLIPSIS.to_owned(), true)
}

/// 按字数估一段文字几个候选字宽（宽字符一个，拉丁字母、数字不到一个）。
pub(super) fn estimated_ems(text: &str) -> f32 {
    text.chars()
        .map(|c| if c.is_ascii() { 0.62 } else { 1.0 })
        .sum()
}

pub(super) fn columns(hdc: HDC, theme: &Theme, rows: &[Row]) -> Columns {
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
            .map(|seg| measure(hdc, theme.annotation_font, &seg.text).cx)
            .sum();
        columns.index_width = columns.index_width.max(index.cx);
        columns.text_width = columns
            .text_width
            .max(text.cx + cloud_prefix_width(hdc, theme, row) + code_width(hdc, theme, row));
        columns.annotation_width = columns.annotation_width.max(annotation);
        columns.row_height = columns.row_height.max(text.cy + theme.row_padding * 2);
    }
    columns
}

/// 小字相对候选词往下挪多少才纵向居中。
pub(super) fn small_offset(hdc: HDC, theme: &Theme, text_height: i32) -> i32 {
    ((text_height - line_height(hdc, theme.annotation_font)) / 2).max(0)
}

/// 云朵字形加它与后面文字的间隔。
pub(super) fn cloud_glyph_width(hdc: HDC, theme: &Theme) -> i32 {
    measure(hdc, theme.annotation_font, CLOUD_GLYPH).cx + theme.column_gap / 2
}

pub(super) fn cloud_prefix_width(hdc: HDC, theme: &Theme, row: &Row) -> i32 {
    if row.cloud {
        cloud_glyph_width(hdc, theme)
    } else {
        0
    }
}

/// 候选词后面那段辅码的宽度；没有码是 0。
pub(super) fn code_width(hdc: HDC, theme: &Theme, row: &Row) -> i32 {
    row.code
        .as_ref()
        .map_or(0, |code| measure(hdc, theme.annotation_font, code).cx)
}

/// 画候选词本体，云端词前带小云朵、后面紧跟辅码。返回占用宽度。
pub(super) fn draw_word(
    hdc: HDC,
    theme: &Theme,
    row: &Row,
    x: i32,
    baseline: i32,
    small_offset: i32,
) -> i32 {
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
    let mut width =
        prefix + draw_text(hdc, theme.text_font, color, x + prefix, baseline, &row.text);
    if let Some(code) = &row.code {
        width += draw_text(
            hdc,
            theme.annotation_font,
            tone_color(theme, Tone::Code),
            x + width,
            baseline + small_offset,
            code,
        );
    }
    width
}

pub(super) fn tone_color(theme: &Theme, tone: Tone) -> COLORREF {
    match tone {
        Tone::Gloss => theme.gloss_color,
        Tone::Fresh => theme.fresh_color,
        Tone::Faint | Tone::Pos => theme.pos_color,
        Tone::Code => theme.gloss_color,
    }
}

/// 一行的注记按「第几条译词」切开：返回 (译词序号, 相对注记起点的偏移, 宽度)。
/// 点译文上屏那一条时用；分隔符不属于任何一条。
pub(super) fn sense_spans(hdc: HDC, theme: &Theme, row: &Row) -> Vec<(usize, i32, i32)> {
    let mut spans: Vec<(usize, i32, i32)> = Vec::new();
    let mut x = 0;
    for seg in &row.annotation {
        let width = measure(hdc, theme.annotation_font, &seg.text).cx;
        if let Some(sense) = seg.sense {
            match spans.last_mut() {
                Some((last, _, span)) if *last == sense => *span += width,
                _ => spans.push((sense, x, width)),
            }
        }
        x += width;
    }
    spans
}

/// 拼音行光标前 `cursor` 个字符。
pub(super) fn concat_before_cursor(preedit: &[(String, PreeditKind)], cursor: usize) -> String {
    preedit
        .iter()
        .flat_map(|(text, _)| text.chars())
        .take(cursor)
        .collect()
}

/// 画一段文字，`(x, y)` 是左上角，返回宽度。调用方先 `SetBkMode(TRANSPARENT)`。
pub(super) fn draw_text(hdc: HDC, font: HFONT, color: COLORREF, x: i32, y: i32, text: &str) -> i32 {
    let utf16: Vec<u16> = text.encode_utf16().collect();
    let mut size = SIZE::default();
    unsafe {
        SelectObject(hdc, font.into());
        SetTextColor(hdc, color);
        let _ = TextOutW(hdc, x, y, &utf16);
        let _ = GetTextExtentPoint32W(hdc, &utf16, &mut size);
    }
    size.cx
}

pub(super) fn measure(hdc: HDC, font: HFONT, text: &str) -> SIZE {
    let utf16: Vec<u16> = text.encode_utf16().collect();
    let mut size = SIZE::default();
    unsafe {
        SelectObject(hdc, font.into());
        let _ = GetTextExtentPoint32W(hdc, &utf16, &mut size);
    }
    size
}

pub(super) fn line_height(hdc: HDC, font: HFONT) -> i32 {
    measure(hdc, font, "x").cy
}

/// 细线 / 光标的粗细，随 DPI 放大。
pub(super) fn scale_line(theme: &Theme) -> i32 {
    (theme.padding / 8).max(1)
}

pub(super) fn fill_rect(hdc: HDC, rect: RECT, color: COLORREF) {
    unsafe {
        let brush = CreateSolidBrush(color);
        FillRect(hdc, &rect, brush);
        let _ = DeleteObject(brush.into());
    }
}

pub(super) fn fill_round_rect(hdc: HDC, rect: RECT, color: COLORREF, radius: i32) {
    let diameter = (radius * 2).max(1);
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
