//! 面板的绘制：分组胶囊（当前组强调色实心、白字）、按键提示与 ✕、分类标签（当前分类加强调色下划线）、
//! 格子（条目居中，高亮格铺底色）、信息行（高亮条目、它的名字与行号）。与 macOS 端的系统绘制一致。

use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::DirectWrite::IDWriteTextFormat;

use super::d2d::{Canvas, Rect};
use super::layout::{Fonts, HINT, PanelLayout};
use crate::dispatch::PanelFrame;

/// 面板用的颜色，取自候选窗的主题加系统强调色。
pub(super) struct Colors {
    pub text: COLORREF,

    pub gloss: COLORREF,

    pub index: COLORREF,

    pub separator: COLORREF,

    pub highlight: COLORREF,

    pub accent: COLORREF,

    /// ✕ 底下的淡色圆。
    pub close: COLORREF,
}

const WHITE: COLORREF = COLORREF(0x00FF_FFFF);

pub(super) fn paint(
    canvas: &Canvas,
    frame: &PanelFrame,
    layout: &PanelLayout,
    fonts: &Fonts,
    colors: &Colors,
) {
    let small = fonts.label_height;
    let (pad_x, pad_y) = layout.pill_pad();
    for (i, (group, rect)) in frame.groups.iter().zip(&layout.pills).enumerate() {
        let active = i == frame.group;
        if active {
            canvas.fill_round(*rect, rect.h / 2.0, colors.accent);
        }
        let color = if active { WHITE } else { colors.gloss };
        canvas.text(
            group,
            &fonts.label,
            color,
            rect.x + pad_x,
            rect.y + pad_y,
            rect.w,
        );
    }

    canvas.fill_circle(layout.close, colors.close);
    canvas.text_centered("✕", &fonts.hint, colors.gloss, layout.close);
    if let Some(first) = layout.pills.last() {
        let start = first.x + first.w + layout.close_gap();
        let end = layout.close.x - layout.close_gap();
        if end > start {
            let hint = Rect::new(start, first.y, end - start, first.h);
            draw_right_aligned(canvas, HINT, &fonts.hint, colors.index, hint);
        }
    }

    for (i, rect) in &layout.tabs {
        let active = *i == frame.section;
        let color = if active { colors.text } else { colors.gloss };
        canvas.text(
            &frame.sections[*i],
            &fonts.label,
            color,
            rect.x,
            rect.y,
            rect.w + 1.0,
        );
        if active {
            let line = Rect::new(
                rect.x,
                rect.y + small + layout.scale,
                rect.w,
                layout.underline(),
            );
            canvas.fill_round(line, line.h / 2.0, colors.accent);
        }
    }
    if let Some((_, first)) = layout.tabs.first()
        && layout.more_left
    {
        canvas.text(
            "‹",
            &fonts.label,
            colors.index,
            layout.left,
            first.y,
            small * 2.0,
        );
    }
    if let Some((_, last)) = layout.tabs.last()
        && layout.more_right
    {
        let marker = Rect::new(layout.right - small, last.y, small, small);
        draw_right_aligned(canvas, "›", &fonts.label, colors.index, marker);
    }
    canvas.fill(layout.divider, colors.separator);

    let item_font = if layout.narrow {
        &fonts.item
    } else {
        &fonts.wide_item
    };
    let inset = 2.0 * layout.scale;
    for (offset, cell) in frame.cells.iter().enumerate() {
        let Some(cell) = cell else {
            continue;
        };
        let rect = layout.cell_rect(offset);
        if offset == frame.highlight {
            canvas.fill_round(rect, 6.0 * layout.scale, colors.highlight);
        }
        let inner = Rect::new(rect.x + inset, rect.y, rect.w - inset * 2.0, rect.h);
        canvas.text_centered(&cell.text, item_font, colors.text, inner);
    }

    // 信息行：左边是高亮条目与它的名字，右边是第几行
    let gap = 8.0 * layout.scale;
    let mut budget = layout.right - layout.left;
    if let Some(footer) = frame.footer.as_deref() {
        let rect = Rect::new(layout.left, layout.info_top, budget, small);
        let width = draw_right_aligned(canvas, footer, &fonts.hint, colors.index, rect);
        budget -= width + gap;
    }
    if let Some(Some(cell)) = frame.cells.get(frame.highlight) {
        let mut x = layout.left;
        let used = canvas.text(
            &cell.text,
            &fonts.label,
            colors.text,
            x,
            layout.info_top,
            budget,
        );
        x += used + gap;
        budget -= used + gap;
        if let Some(name) = cell.name.as_deref()
            && budget > 0.0
        {
            canvas.text(name, &fonts.label, colors.gloss, x, layout.info_top, budget);
        }
    }
}

/// 贴着 `rect` 右边、竖直居中画一行字；返回字宽。
fn draw_right_aligned(
    canvas: &Canvas,
    text: &str,
    format: &IDWriteTextFormat,
    color: COLORREF,
    rect: Rect,
) -> f32 {
    let (width, height) = canvas.measure(text, format);
    let width = width.min(rect.w);
    canvas.text(
        text,
        format,
        color,
        rect.x + rect.w - width,
        rect.y + (rect.h - height) / 2.0,
        width + 1.0,
    );
    width
}
