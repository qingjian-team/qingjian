//! 一帧的测量与绘制：GDI 直接画到内存位图上，落笔的同时记下命中区。
//!
//! 布局沿用 Server 候选窗的 GDI 画法（`apps/windows/server/src/ui/candidates/view.rs`）：
//! 竖排 `序号  候选词   读音·词性 译文`，横排左右排、高亮项的译文另起一行，横排展开成矩阵时一行 `columns` 格。
//! 这一份只服务于 TSF 进程内的候选窗，不改 Server 那套。

use windows::Win32::Foundation::{RECT, SIZE};
use windows::Win32::Graphics::Gdi::{HDC, SetBkMode, TRANSPARENT};

use qingjian_platform::LayoutMode;
use qingjian_platform::protocol::PreeditKind;

use self::gdi::{
    cloud_glyph_width, cloud_prefix_width, code_width, columns, concat_before_cursor, draw_text,
    draw_word, fill_rect, fill_round_rect, line_height, measure, scale_line, sense_spans,
    small_offset, tone_color,
};
use self::matrix::{draw_matrix, matrix_size};
use super::theme::Theme;
use super::view::{HitRegion, HitTarget, View};

mod gdi;
mod matrix;
#[cfg(test)]
mod tests;

/// 云端候选词前的小云朵。
const CLOUD_GLYPH: &str = "☁";

/// 竖排的列宽与统一行高。
struct Columns {
    index_width: i32,
    text_width: i32,
    annotation_width: i32,
    row_height: i32,
}

/// 内容需要的大小（含内边距）。
pub(crate) fn preferred_size(hdc: HDC, theme: &Theme, view: &View) -> SIZE {
    let (top_width, top_height) = top_line_size(hdc, theme, view);
    let (notice_width, notice_height) = notice_line_size(hdc, theme, view);
    let (body_width, body_height) = match view.layout {
        LayoutMode::Vertical => vertical_size(hdc, theme, view),
        LayoutMode::Horizontal if view.columns > 0 => matrix_size(hdc, theme, view),
        LayoutMode::Horizontal => horizontal_size(hdc, theme, view),
    };
    SIZE {
        cx: top_width.max(body_width).max(notice_width) + theme.padding * 2,
        cy: top_height + notice_height + body_height + theme.padding * 2,
    }
}

/// 画整帧，返回命中区（内容区像素）。背景与描边由 [`Canvas::fill_background`](super::canvas::Canvas) 铺好。
pub(crate) fn paint(hdc: HDC, theme: &Theme, view: &View, client: RECT) -> Vec<HitRegion> {
    let _ = unsafe { SetBkMode(hdc, TRANSPARENT) };
    let width = client.right - client.left;
    let mut hits = Vec::new();
    let mut y = theme.padding;
    y += draw_top_line(hdc, theme, view, y);
    y += draw_notice(hdc, theme, view, y);
    match view.layout {
        LayoutMode::Vertical => draw_rows(hdc, theme, view, y, width, &mut hits),
        LayoutMode::Horizontal if view.columns > 0 => {
            draw_matrix(hdc, theme, view, y, width, &mut hits)
        }
        LayoutMode::Horizontal => draw_horizontal(hdc, theme, view, y, width, &mut hits),
    }
    hits
}

fn vertical_size(hdc: HDC, theme: &Theme, view: &View) -> (i32, i32) {
    if view.rows.is_empty() {
        return (0, 0);
    }
    let columns = columns(hdc, theme, &view.rows);
    let mut body_width = columns.index_width + theme.column_gap + columns.text_width;
    if columns.annotation_width > 0 {
        body_width += theme.column_gap + columns.annotation_width;
    }
    let mut body_height = columns.row_height * view.rows.len() as i32;
    if let Some(footer) = &view.footer {
        let size = measure(hdc, theme.index_font, footer);
        body_width = body_width.max(size.cx);
        body_height += size.cy + theme.row_padding;
    }
    (body_width, body_height)
}

fn horizontal_size(hdc: HDC, theme: &Theme, view: &View) -> (i32, i32) {
    if view.rows.is_empty() {
        return (0, 0);
    }
    let index_gap = theme.column_gap / 2;
    let highlight_inset = theme.padding / 2;
    let mut row_height = 0;
    let mut width = 0;
    for row in &view.rows {
        let index = measure(hdc, theme.index_font, &row.index);
        let text = measure(hdc, theme.text_font, &row.text);
        width += index.cx
            + index_gap
            + cloud_prefix_width(hdc, theme, row)
            + text.cx
            + code_width(hdc, theme, row);
        row_height = row_height.max(text.cy + theme.row_padding * 2);
    }
    width += theme.column_gap * (view.rows.len().saturating_sub(1)) as i32 + highlight_inset * 2;
    if let Some(footer) = &view.footer {
        width += theme.column_gap + measure(hdc, theme.index_font, footer).cx;
    }
    let mut height = row_height;
    if let Some((annotation_width, annotation_height)) =
        highlighted_annotation_size(hdc, theme, view)
    {
        width = width.max(annotation_width);
        height += annotation_height;
    }
    (width, height)
}

/// 横排时高亮候选的译文行尺寸；没有译文 / 没有高亮为 `None`。
fn highlighted_annotation_size(hdc: HDC, theme: &Theme, view: &View) -> Option<(i32, i32)> {
    let row = view.rows.get(view.highlight)?;
    if row.annotation.is_empty() {
        return None;
    }
    let width: i32 = row
        .annotation
        .iter()
        .map(|seg| measure(hdc, theme.annotation_font, &seg.text).cx)
        .sum();
    let height = line_height(hdc, theme.annotation_font) + theme.row_padding;
    Some((width, height))
}

fn top_line_size(hdc: HDC, theme: &Theme, view: &View) -> (i32, i32) {
    if view.preedit.is_empty() && view.sentence.is_none() {
        return (0, 0);
    }
    let height = line_height(hdc, theme.annotation_font);
    // 没有拼音行时那段宽度为 0，但整句补全前面的间隔照旧。
    let mut width = if view.preedit.is_empty() {
        0
    } else {
        let full: String = view.preedit.iter().map(|(text, _)| text.as_str()).collect();
        measure(hdc, theme.annotation_font, &full).cx + scale_line(theme)
    };
    if let Some(sentence) = &view.sentence {
        width += theme.column_gap
            + cloud_glyph_width(hdc, theme)
            + measure(hdc, theme.annotation_font, sentence).cx;
    }
    (width, height + theme.row_padding * 2)
}

fn notice_line_size(hdc: HDC, theme: &Theme, view: &View) -> (i32, i32) {
    let Some(notice) = &view.notice else {
        return (0, 0);
    };
    let width = measure(hdc, theme.annotation_font, notice).cx;
    (
        width,
        line_height(hdc, theme.annotation_font) + theme.row_padding,
    )
}

/// 顶部拼音行：各段按样式画、自己画光标、右侧整句补全。返回占用高度。
fn draw_top_line(hdc: HDC, theme: &Theme, view: &View, y: i32) -> i32 {
    if view.preedit.is_empty() && view.sentence.is_none() {
        return 0;
    }
    let height = line_height(hdc, theme.annotation_font);
    let top = y + theme.row_padding;
    let mut x = theme.padding;
    for (text, kind) in &view.preedit {
        let (color, strike, underline) = match kind {
            PreeditKind::Typed => (theme.gloss_color, false, false),
            PreeditKind::Rest => (theme.pos_color, false, false),
            PreeditKind::Corrected => (theme.pos_color, true, false),
            // 辅码码段：与剩余拼音同一个淡色，再压一道下划线把它们区分开
            PreeditKind::AuxCode => (theme.pos_color, false, true),
        };
        let width = draw_text(hdc, theme.annotation_font, color, x, top, text);
        let weight = scale_line(theme);
        if strike {
            fill_rect(
                hdc,
                RECT {
                    left: x,
                    top: top + height / 2,
                    right: x + width,
                    bottom: top + height / 2 + weight,
                },
                theme.pos_color,
            );
        }
        if underline {
            fill_rect(
                hdc,
                RECT {
                    left: x,
                    top: top + height,
                    right: x + width,
                    bottom: top + height + weight,
                },
                theme.pos_color,
            );
        }
        x += width;
    }
    if !view.preedit.is_empty() {
        let before = concat_before_cursor(&view.preedit, view.cursor);
        let caret_x = theme.padding + measure(hdc, theme.annotation_font, &before).cx;
        fill_rect(
            hdc,
            RECT {
                left: caret_x,
                top,
                right: caret_x + scale_line(theme),
                bottom: top + height,
            },
            theme.text_color,
        );
    }
    if let Some(sentence) = &view.sentence {
        let sentence_x = x + theme.column_gap;
        let cloud = cloud_glyph_width(hdc, theme);
        draw_text(
            hdc,
            theme.annotation_font,
            theme.cloud_color,
            sentence_x,
            top,
            CLOUD_GLYPH,
        );
        draw_text(
            hdc,
            theme.annotation_font,
            theme.gloss_color,
            sentence_x + cloud,
            top,
            sentence,
        );
    }
    height + theme.row_padding * 2
}

/// 屏幕提示行：拼音行下方、候选行上方，淡色小字。返回占用高度。
fn draw_notice(hdc: HDC, theme: &Theme, view: &View, y: i32) -> i32 {
    let Some(notice) = &view.notice else {
        return 0;
    };
    draw_text(
        hdc,
        theme.annotation_font,
        theme.pos_color,
        theme.padding,
        y,
        notice,
    );
    line_height(hdc, theme.annotation_font) + theme.row_padding
}

fn draw_rows(hdc: HDC, theme: &Theme, view: &View, y: i32, width: i32, hits: &mut Vec<HitRegion>) {
    let columns = columns(hdc, theme, &view.rows);
    let text_x = theme.padding + columns.index_width + theme.column_gap;
    let annotation_x = text_x + columns.text_width + theme.column_gap;
    let total = columns.row_height * view.rows.len() as i32;
    for (i, row) in view.rows.iter().enumerate() {
        // 倒着排：第一项贴着光标（窗口下方），序号仍跟着候选走
        let row_y = if view.reversed {
            y + total - columns.row_height * (i as i32 + 1)
        } else {
            y + columns.row_height * i as i32
        };
        if i == view.highlight {
            fill_round_rect(
                hdc,
                RECT {
                    left: theme.padding / 2,
                    top: row_y,
                    right: width - theme.padding / 2,
                    bottom: row_y + columns.row_height,
                },
                theme.highlight,
                theme.corner_radius / 2,
            );
        }
        let baseline = row_y + theme.row_padding;
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
        for seg in &row.annotation {
            x += draw_text(
                hdc,
                theme.annotation_font,
                tone_color(theme, seg.tone),
                x,
                baseline + small_offset,
                &seg.text,
            );
        }
        // 译词那几块先记，整行后记：点译文上屏，点其余地方选候选。
        for (sense, start, span) in sense_spans(hdc, theme, row) {
            hits.push(HitRegion {
                x: (annotation_x + start) as f32,
                y: row_y as f32,
                width: span as f32,
                height: columns.row_height as f32,
                target: HitTarget::Translation {
                    candidate: i,
                    sense,
                },
            });
        }
        hits.push(HitRegion {
            x: (theme.padding / 2) as f32,
            y: row_y as f32,
            width: (width - theme.padding) as f32,
            height: columns.row_height as f32,
            target: HitTarget::Candidate(i),
        });
    }
    if let Some(footer) = &view.footer {
        let size = measure(hdc, theme.index_font, footer);
        draw_text(
            hdc,
            theme.index_font,
            theme.index_color,
            width - theme.padding - size.cx,
            y + total + theme.row_padding,
            footer,
        );
    }
}

fn draw_horizontal(
    hdc: HDC,
    theme: &Theme,
    view: &View,
    y: i32,
    width: i32,
    hits: &mut Vec<HitRegion>,
) {
    if view.rows.is_empty() {
        return;
    }
    let index_gap = theme.column_gap / 2;
    let highlight_inset = theme.padding / 2;
    let row_height = view
        .rows
        .iter()
        .map(|row| measure(hdc, theme.text_font, &row.text).cy + theme.row_padding * 2)
        .max()
        .unwrap_or(0);
    let baseline = y + theme.row_padding;
    let mut x = theme.padding + highlight_inset;
    for (i, row) in view.rows.iter().enumerate() {
        let index_width = measure(hdc, theme.index_font, &row.index).cx;
        let text_size = measure(hdc, theme.text_font, &row.text);
        let item_width = index_width
            + index_gap
            + cloud_prefix_width(hdc, theme, row)
            + text_size.cx
            + code_width(hdc, theme, row);
        if i == view.highlight {
            fill_round_rect(
                hdc,
                RECT {
                    left: x - highlight_inset,
                    top: y,
                    right: x + item_width + highlight_inset,
                    bottom: y + row_height,
                },
                theme.highlight,
                theme.corner_radius / 2,
            );
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
        hits.push(HitRegion {
            x: (x - highlight_inset) as f32,
            y: y as f32,
            width: (item_width + highlight_inset * 2) as f32,
            height: row_height as f32,
            target: HitTarget::Candidate(i),
        });
        x += item_width + theme.column_gap;
    }
    if let Some(footer) = &view.footer {
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
    if let Some(row) = view.rows.get(view.highlight) {
        let mut x = theme.padding + highlight_inset;
        let top = y + row_height + theme.row_padding / 2;
        for seg in &row.annotation {
            x += draw_text(
                hdc,
                theme.annotation_font,
                tone_color(theme, seg.tone),
                x,
                top,
                &seg.text,
            );
        }
        // 译文行单独记一块：点译文上屏那一条
        for (sense, start, span) in sense_spans(hdc, theme, row) {
            hits.push(HitRegion {
                x: (theme.padding + highlight_inset + start) as f32,
                y: top as f32,
                width: span as f32,
                height: (line_height(hdc, theme.annotation_font)) as f32,
                target: HitTarget::Translation {
                    candidate: view.highlight,
                    sense,
                },
            });
        }
    }
}
