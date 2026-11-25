//! 一帧的测量与绘制：GDI 直接画到内存位图上，落笔的同时记下命中区。
//!
//! 布局沿用 Server 候选窗的 GDI 画法（`apps/windows/server/src/ui/candidates/view.rs`）：
//! 竖排 `序号  候选词   读音·词性 译文`，横排左右排、高亮项的译文另起一行，横排展开成矩阵时一行 `columns` 格。
//! 这一份只服务于 TSF 进程内的候选窗，不改 Server 那套。

use windows::Win32::Foundation::{COLORREF, RECT, SIZE};
use windows::Win32::Graphics::Gdi::{
    CreateRoundRectRgn, CreateSolidBrush, DeleteObject, FillRect, FillRgn, GetTextExtentPoint32W,
    HDC, HFONT, SelectObject, SetBkMode, SetTextColor, TRANSPARENT, TextOutW,
};

use qingjian_platform::LayoutMode;
use qingjian_platform::protocol::PreeditKind;

use super::row::{Row, Tone};
use super::theme::Theme;
use super::view::{HitRegion, HitTarget, View};

/// 云端候选词前的小云朵。
const CLOUD_GLYPH: &str = "☁";

/// 矩阵里一格里候选词最多多宽（按候选字宽的倍数；帧没给列宽时用）。
const MAX_CELL_EMS: f32 = 4.0;

/// 超宽候选截尾补的省略号。
const ELLIPSIS: &str = "…";

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

/// 量好的矩阵网格：每格显示文字（可能已截断）、各列宽度、统一行高。
struct MatrixCells {
    texts: Vec<(String, bool)>,
    index_width: i32,
    column_widths: Vec<i32>,
    row_height: i32,
}

impl MatrixCells {
    /// 第 `column` 列左边相对网格起点的偏移。
    fn offset(&self, column: usize, gap: i32) -> i32 {
        self.column_widths[..column].iter().sum::<i32>() + gap * column as i32
    }

    /// 网格总宽（不含两侧高亮留边）。
    fn width(&self, gap: i32) -> i32 {
        self.column_widths.iter().sum::<i32>()
            + gap * self.column_widths.len().saturating_sub(1) as i32
    }
}

fn matrix_size(hdc: HDC, theme: &Theme, view: &View) -> (i32, i32) {
    if view.rows.is_empty() {
        return (0, 0);
    }
    let cells = matrix_cells(hdc, theme, view);
    let grid_rows = view.rows.len().div_ceil(view.columns.max(1));
    let info_height = line_height(hdc, theme.annotation_font) + theme.row_padding;
    (
        cells.width(theme.column_gap) + theme.padding / 2 * 2,
        cells.row_height * grid_rows as i32 + info_height,
    )
}

fn draw_matrix(
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
    let cells = matrix_cells(hdc, theme, view);
    let columns = view.columns.max(1);
    let text_height = measure(hdc, theme.text_font, "国").cy;
    let inset = theme.padding / 2;
    let index_gap = theme.column_gap / 2;
    let origin = theme.padding + inset;
    for (i, row) in view.rows.iter().enumerate() {
        let (text, _) = &cells.texts[i];
        if text.is_empty() && row.index.is_empty() {
            continue;
        }
        let cell_width = cells.column_widths[i % columns];
        let x = origin + cells.offset(i % columns, theme.column_gap);
        let row_y = y + cells.row_height * (i / columns) as i32;
        let baseline = row_y + theme.row_padding;
        if i == view.highlight {
            fill_round_rect(
                hdc,
                RECT {
                    left: x - inset,
                    top: row_y,
                    right: x + cell_width + inset,
                    bottom: row_y + cells.row_height,
                },
                theme.highlight,
                theme.corner_radius / 2,
            );
        }
        let small_offset = small_offset(hdc, theme, text_height);
        if !row.index.is_empty() {
            draw_text(
                hdc,
                theme.index_font,
                theme.index_color,
                x,
                baseline + small_offset,
                &row.index,
            );
        }
        let mut shown = row.clone();
        shown.text.clone_from(text);
        draw_word(
            hdc,
            theme,
            &shown,
            x + cells.index_width + index_gap,
            baseline,
            small_offset,
        );
        // 空位不点
        if !row.index.is_empty() || !text.is_empty() {
            hits.push(HitRegion {
                x: (x - inset) as f32,
                y: row_y as f32,
                width: (cell_width + inset * 2) as f32,
                height: cells.row_height as f32,
                target: HitTarget::Candidate(i),
            });
        }
    }
    // 信息行：页码靠右；左边先放被截断的高亮候选的完整文本，再放译文，放不下的截断
    let grid_rows = view.rows.len().div_ceil(columns);
    let info_top = y + cells.row_height * grid_rows as i32 + theme.row_padding / 2;
    let right = width - theme.padding;
    let info_gap = theme.padding * 5 / 4;
    let mut budget = right - origin;
    if let Some(footer) = &view.footer {
        let size = measure(hdc, theme.index_font, footer);
        draw_text(
            hdc,
            theme.index_font,
            theme.index_color,
            right - size.cx,
            info_top,
            footer,
        );
        budget -= size.cx + theme.column_gap;
    }
    let mut x = origin;
    let Some(row) = (view.highlight != usize::MAX)
        .then(|| view.rows.get(view.highlight))
        .flatten()
    else {
        return;
    };
    if cells.texts[view.highlight].1 {
        let used = draw_clipped(hdc, theme, &row.text, theme.text_color, x, info_top, budget);
        x += used + info_gap;
        budget -= used + info_gap;
    }
    for seg in &row.annotation {
        if budget <= 0 {
            break;
        }
        let used = draw_clipped(
            hdc,
            theme,
            &seg.text,
            tone_color(theme, seg.tone),
            x,
            info_top,
            budget,
        );
        x += used;
        budget -= used;
    }
}

/// 信息行里画一段小字，宽度超过 `budget` 就截断；返回画了多宽。
fn draw_clipped(
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

/// 每格的显示文字与各列宽度。序号列按最宽的一位数留，行与行才对得齐。
fn matrix_cells(hdc: HDC, theme: &Theme, view: &View) -> MatrixCells {
    let columns = view.columns.max(1);
    let em = measure(hdc, theme.text_font, "国").cx;
    let index_width = measure(hdc, theme.index_font, "8").cx;
    let slack = theme.padding * 3 / 16;
    let index_gap = theme.column_gap / 2;
    let fixed: Option<Vec<i32>> = (view.column_ems.len() == columns).then(|| {
        view.column_ems
            .iter()
            .map(|ems| (*ems * em as f32 + slack as f32) as i32)
            .collect()
    });
    let mut text_widths = fixed.clone().unwrap_or_else(|| vec![0; columns]);
    let row_height = measure(hdc, theme.text_font, "国").cy + theme.row_padding * 2;
    let texts = view
        .rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let column = i % columns;
            let cloud = cloud_prefix_width(hdc, theme, row);
            let limit = fixed
                .as_ref()
                .map_or(MAX_CELL_EMS * em as f32, |widths| widths[column] as f32);
            if fixed.is_some() && estimated_ems(&row.text) * em as f32 + cloud as f32 <= limit {
                return (row.text.clone(), false);
            }
            let (text, truncated) = truncate(
                hdc,
                theme.text_font,
                &row.text,
                (limit - cloud as f32).max(0.0),
            );
            if fixed.is_none() {
                let width = (measure(hdc, theme.text_font, &text).cx + cloud).max(0);
                text_widths[column] = text_widths[column].max(width);
            }
            (text, truncated)
        })
        .collect();
    MatrixCells {
        texts,
        index_width,
        column_widths: text_widths
            .iter()
            .map(|width| index_width + index_gap + width)
            .collect(),
        row_height,
    }
}

/// `text` 宽度超过 `max_width` 就从末尾去字、补上「…」直到放得下；返回显示文字与是否截断过。
fn truncate(hdc: HDC, font: HFONT, text: &str, max_width: f32) -> (String, bool) {
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
fn estimated_ems(text: &str) -> f32 {
    text.chars()
        .map(|c| if c.is_ascii() { 0.62 } else { 1.0 })
        .sum()
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
fn small_offset(hdc: HDC, theme: &Theme, text_height: i32) -> i32 {
    ((text_height - line_height(hdc, theme.annotation_font)) / 2).max(0)
}

/// 云朵字形加它与后面文字的间隔。
fn cloud_glyph_width(hdc: HDC, theme: &Theme) -> i32 {
    measure(hdc, theme.annotation_font, CLOUD_GLYPH).cx + theme.column_gap / 2
}

fn cloud_prefix_width(hdc: HDC, theme: &Theme, row: &Row) -> i32 {
    if row.cloud {
        cloud_glyph_width(hdc, theme)
    } else {
        0
    }
}

/// 候选词后面那段辅码的宽度；没有码是 0。
fn code_width(hdc: HDC, theme: &Theme, row: &Row) -> i32 {
    row.code
        .as_ref()
        .map_or(0, |code| measure(hdc, theme.annotation_font, code).cx)
}

/// 画候选词本体，云端词前带小云朵、后面紧跟辅码。返回占用宽度。
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

fn tone_color(theme: &Theme, tone: Tone) -> COLORREF {
    match tone {
        Tone::Gloss => theme.gloss_color,
        Tone::Fresh => theme.fresh_color,
        Tone::Faint | Tone::Pos => theme.pos_color,
        Tone::Code => theme.gloss_color,
    }
}

/// 一行的注记按「第几条译词」切开：返回 (译词序号, 相对注记起点的偏移, 宽度)。
/// 点译文上屏那一条时用；分隔符不属于任何一条。
fn sense_spans(hdc: HDC, theme: &Theme, row: &Row) -> Vec<(usize, i32, i32)> {
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
fn concat_before_cursor(preedit: &[(String, PreeditKind)], cursor: usize) -> String {
    preedit
        .iter()
        .flat_map(|(text, _)| text.chars())
        .take(cursor)
        .collect()
}

/// 画一段文字，`(x, y)` 是左上角，返回宽度。调用方先 `SetBkMode(TRANSPARENT)`。
fn draw_text(hdc: HDC, font: HFONT, color: COLORREF, x: i32, y: i32, text: &str) -> i32 {
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

fn measure(hdc: HDC, font: HFONT, text: &str) -> SIZE {
    let utf16: Vec<u16> = text.encode_utf16().collect();
    let mut size = SIZE::default();
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
    unsafe {
        let brush = CreateSolidBrush(color);
        FillRect(hdc, &rect, brush);
        let _ = DeleteObject(brush.into());
    }
}

fn fill_round_rect(hdc: HDC, rect: RECT, color: COLORREF, radius: i32) {
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

#[cfg(test)]
mod tests {
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
        let file =
            std::fs::File::create(std::env::temp_dir().join("qingjian-candidate-canvas.png"))
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
                        .chunks_exact(4)
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
}
