//! 横排展开成矩阵时的量尺寸与绘制：一行 `columns` 格，序号列按最宽的一位数对齐。

use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Gdi::HDC;

use super::gdi::{
    cloud_prefix_width, draw_clipped, draw_text, draw_word, estimated_ems, fill_round_rect,
    line_height, measure, small_offset, tone_color, truncate,
};
use crate::com::candidates::paint::theme::Theme;
use crate::com::candidates::paint::view::{HitRegion, HitTarget, View};

/// 矩阵里一格里候选词最多多宽（按候选字宽的倍数；帧没给列宽时用）。
const MAX_CELL_EMS: f32 = 4.0;

/// 量好的矩阵网格：每格显示文字（可能已截断）、各列宽度、统一行高。
pub(super) struct MatrixCells {
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

pub(super) fn matrix_size(hdc: HDC, theme: &Theme, view: &View) -> (i32, i32) {
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

pub(super) fn draw_matrix(
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

/// 每格的显示文字与各列宽度。序号列按最宽的一位数留，行与行才对得齐。
pub(super) fn matrix_cells(hdc: HDC, theme: &Theme, view: &View) -> MatrixCells {
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
