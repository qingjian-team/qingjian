//! 候选窗口的绘制与测量。布局镜像 macOS 端：竖排 `序号  候选词   读音·词性 译文`，
//! 横排 `序号 候选词` 左右排、高亮项的译文另起一行；顶部一行拼音，右侧整句补全。

use qingjian_platform::LayoutMode;
use qingjian_platform::protocol::PreeditKind;
use qingjian_render::{Row, Tone};
use windows::Win32::Foundation::{COLORREF, RECT, SIZE};
use windows::Win32::Graphics::Gdi::{
    CreateRoundRectRgn, CreateSolidBrush, DeleteObject, FillRect, FillRgn, GetTextExtentPoint32W,
    HDC, HFONT, SelectObject, SetBkMode, SetTextColor, TRANSPARENT, TextOutW,
};

use super::RenderData;
use super::theme::Theme;

/// 云端候选词前的小云朵（macOS 用 SF Symbol `cloud`）。
const CLOUD_GLYPH: &str = "☁";

/// 矩阵里一格里候选词最多多宽（按候选字宽的倍数；帧没给列宽时用），与渲染器一致。
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
pub(super) fn preferred_size(hdc: HDC, data: &RenderData) -> SIZE {
    let theme = &data.theme;
    let (top_width, top_height) = top_line_size(hdc, data);
    let (notice_width, notice_height) = notice_line_size(hdc, data);
    let (body_width, body_height) = match data.layout {
        LayoutMode::Vertical => vertical_size(hdc, data),
        LayoutMode::Horizontal if data.columns > 0 => matrix_size(hdc, data),
        LayoutMode::Horizontal => horizontal_size(hdc, data),
    };
    SIZE {
        cx: top_width.max(body_width).max(notice_width) + theme.padding * 2,
        cy: top_height + notice_height + body_height + theme.padding * 2,
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
        width += index.cx
            + index_gap
            + cloud_prefix_width(hdc, theme, row)
            + text.cx
            + code_width(hdc, theme, row);
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

/// 横排时高亮候选的译文行尺寸；没有译文 / 没有高亮为 `None`。
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

/// 画整帧；背景与阴影已由合成器铺好，`client` 是内容区。
pub(super) fn paint(hdc: HDC, data: &RenderData, client: RECT) {
    let theme = &data.theme;
    unsafe { SetBkMode(hdc, TRANSPARENT) };

    let mut y = theme.padding;
    y += draw_top_line(hdc, data, y);
    y += draw_notice(hdc, data, y);
    match data.layout {
        LayoutMode::Vertical => draw_rows(hdc, data, y, client.right - client.left),
        LayoutMode::Horizontal if data.columns > 0 => {
            draw_matrix(hdc, data, y, client.right - client.left)
        }
        LayoutMode::Horizontal => draw_horizontal(hdc, data, y, client.right - client.left),
    }
}

/// 顶部拼音行：各段按样式画、自己画光标、右侧整句补全。返回占用高度。
/// 「只在行内」时没有拼音行，但整句补全仍要画（占用同一条线）。
fn draw_top_line(hdc: HDC, data: &RenderData, y: i32) -> i32 {
    if data.preedit.is_empty() && data.sentence.is_none() {
        return 0;
    }
    let theme = &data.theme;
    let height = line_height(hdc, theme.annotation_font);
    let top = y + theme.row_padding;
    let mut x = theme.padding;
    for (text, kind) in &data.preedit {
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
            let line = RECT {
                left: x,
                top: top + height / 2,
                right: x + width,
                bottom: top + height / 2 + weight,
            };
            fill_rect(hdc, line, theme.pos_color);
        }
        if underline {
            let line = RECT {
                left: x,
                top: top + height,
                right: x + width,
                bottom: top + height + weight,
            };
            fill_rect(hdc, line, theme.pos_color);
        }
        x += width;
    }
    if !data.preedit.is_empty() {
        let before = concat_before_cursor(&data.preedit, data.cursor);
        let caret_x = theme.padding + measure(hdc, theme.annotation_font, &before).cx;
        let caret = RECT {
            left: caret_x,
            top,
            right: caret_x + scale_line(theme),
            bottom: top + height,
        };
        fill_rect(hdc, caret, theme.text_color);
    }
    if let Some(sentence) = &data.sentence {
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
fn draw_notice(hdc: HDC, data: &RenderData, y: i32) -> i32 {
    let Some(notice) = &data.notice else {
        return 0;
    };
    let theme = &data.theme;
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

fn notice_line_size(hdc: HDC, data: &RenderData) -> (i32, i32) {
    let Some(notice) = &data.notice else {
        return (0, 0);
    };
    let theme = &data.theme;
    let width = measure(hdc, theme.annotation_font, notice).cx;
    (
        width,
        line_height(hdc, theme.annotation_font) + theme.row_padding,
    )
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
        let item_width = index_width
            + index_gap
            + cloud_prefix_width(hdc, theme, row)
            + text_size.cx
            + code_width(hdc, theme, row);
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

/// 量好的矩阵网格（镜像渲染器的 `renderer/matrix.rs`）：每格显示文字（可能已截断）、各列宽度、统一行高。
struct MatrixCells {
    texts: Vec<(String, bool)>,

    index_width: i32,

    /// 每列一格的宽度（序号 + 间距 + 候选词）。
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

/// 矩阵：横排展开后的多行网格。一行 `columns` 格，各列宽度由帧给（按整份候选估的，滚动时不变），
/// 超宽候选截尾加「…」；网格下面固定留一行信息：高亮候选被截断时的完整文本、它的译文，页码靠右。
/// 高亮怎么移、视口怎么滚，窗口都不跳。
fn matrix_size(hdc: HDC, data: &RenderData) -> (i32, i32) {
    if data.rows.is_empty() {
        return (0, 0);
    }
    let theme = &data.theme;
    let cells = matrix_cells(hdc, data);
    let grid_rows = data.rows.len().div_ceil(data.columns.max(1));
    let info_height = line_height(hdc, theme.annotation_font) + theme.row_padding;
    (
        cells.width(theme.column_gap) + theme.padding / 2 * 2,
        cells.row_height * grid_rows as i32 + info_height,
    )
}

fn draw_matrix(hdc: HDC, data: &RenderData, y: i32, width: i32) {
    if data.rows.is_empty() {
        return;
    }
    let theme = &data.theme;
    let cells = matrix_cells(hdc, data);
    let columns = data.columns.max(1);
    let text_height = measure(hdc, theme.text_font, "国").cy;
    let inset = theme.padding / 2;
    let index_gap = theme.column_gap / 2;
    let origin = theme.padding + inset;
    for (i, row) in data.rows.iter().enumerate() {
        let (text, _) = &cells.texts[i];
        if text.is_empty() && row.index.is_empty() {
            continue;
        }
        let cell_width = cells.column_widths[i % columns];
        let x = origin + cells.offset(i % columns, theme.column_gap);
        let row_y = y + cells.row_height * (i / columns) as i32;
        let baseline = row_y + theme.row_padding;
        if i == data.highlight {
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
    }
    // 信息行：页码靠右；左边先放被截断的高亮候选的完整文本，再放译文，放不下的截断
    let grid_rows = data.rows.len().div_ceil(columns);
    let info_top = y + cells.row_height * grid_rows as i32 + theme.row_padding / 2;
    let right = width - theme.padding;
    // 译文与完整文本的间距（渲染器 10 点，按 padding 的 DPI 比例换算）
    let info_gap = theme.padding * 5 / 4;
    let mut budget = right - origin;
    if let Some(footer) = &data.footer {
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
    let Some(row) = (data.highlight != usize::MAX)
        .then(|| data.rows.get(data.highlight))
        .flatten()
    else {
        return;
    };
    if cells.texts[data.highlight].1 {
        let used = draw_clipped(hdc, theme, &row.text, theme.text_color, x, info_top, budget);
        x += used + info_gap;
        budget -= used + info_gap;
    }
    for (segment, tone) in &row.annotation {
        if budget <= 0 {
            break;
        }
        let used = draw_clipped(
            hdc,
            theme,
            segment,
            tone_color(theme, *tone),
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
/// 列宽优先用帧给的（按整份候选估的，滚动时不变）；没给就按视口里实测、每格封顶 [`MAX_CELL_EMS`]。
fn matrix_cells(hdc: HDC, data: &RenderData) -> MatrixCells {
    let theme = &data.theme;
    let columns = data.columns.max(1);
    let em = measure(hdc, theme.text_font, "国").cx;
    let index_width = measure(hdc, theme.index_font, "8").cx;
    // 列宽在估出来的字宽之外再留一点（渲染器 1.5 点，按 padding 的 DPI 比例换算）
    let slack = theme.padding * 3 / 16;
    let index_gap = theme.column_gap / 2;
    let fixed: Option<Vec<i32>> = (data.column_ems.len() == columns).then(|| {
        data.column_ems
            .iter()
            .map(|ems| (*ems * em as f32 + slack as f32) as i32)
            .collect()
    });
    let mut text_widths = fixed.clone().unwrap_or_else(|| vec![0; columns]);
    let row_height = measure(hdc, theme.text_font, "国").cy + theme.row_padding * 2;
    let texts = data
        .rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let column = i % columns;
            let cloud = cloud_prefix_width(hdc, theme, row);
            let limit = fixed
                .as_ref()
                .map_or(MAX_CELL_EMS * em as f32, |widths| widths[column] as f32);
            // 列宽是按字数估出来的：同样按字数估着放得下的格子不用再实测截断
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

/// 按字数估一段文字几个候选字宽（宽字符一个，拉丁字母、数字不到一个），
/// 与 Core `Grid::column_ems`、渲染器同一条规则。
fn estimated_ems(text: &str) -> f32 {
    text.chars()
        .map(|c| if c.is_ascii() { 0.62 } else { 1.0 })
        .sum()
}

fn top_line_size(hdc: HDC, data: &RenderData) -> (i32, i32) {
    if data.preedit.is_empty() && data.sentence.is_none() {
        return (0, 0);
    }
    let theme = &data.theme;
    let height = line_height(hdc, theme.annotation_font);
    // 没有拼音行时那段宽度为 0，但整句补全前面的间隔照旧。
    let mut width = if data.preedit.is_empty() {
        0
    } else {
        let full: String = data.preedit.iter().map(|(t, _)| t.as_str()).collect();
        measure(hdc, theme.annotation_font, &full).cx + scale_line(theme)
    };
    if let Some(sentence) = &data.sentence {
        width += theme.column_gap
            + cloud_glyph_width(hdc, theme)
            + measure(hdc, theme.annotation_font, sentence).cx;
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
            .max(text.cx + cloud_prefix_width(hdc, theme, row) + code_width(hdc, theme, row));
        columns.annotation_width = columns.annotation_width.max(annotation);
        columns.row_height = columns.row_height.max(text.cy + theme.row_padding * 2);
    }
    columns
}

/// 小字相对候选词往下挪多少才纵向居中（GDI y 向下，取一半差）。
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
        Tone::Faint => theme.pos_color,
        Tone::Code => theme.gloss_color,
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

/// 画一段文字，`(x, y)` 是左上角，返回宽度。调用方先 `SetBkMode(TRANSPARENT)`。
pub(crate) fn draw_text(hdc: HDC, font: HFONT, color: COLORREF, x: i32, y: i32, text: &str) -> i32 {
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

pub(crate) fn measure(hdc: HDC, font: HFONT, text: &str) -> SIZE {
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

pub(crate) fn fill_rect(hdc: HDC, rect: RECT, color: COLORREF) {
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
