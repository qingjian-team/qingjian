//! 符号面板的系统绘制：顶上分组胶囊与分类标签，中间等宽格子（条目居中），底下信息行（高亮条目的名字、行号）。
//! 青简渲染器还没有面板，开着它时面板也走这里。画与点击共用 [`PanelLayout`] 算出的位置。

use objc2::DefinedClass;
use objc2::rc::Retained;
use objc2_app_kit::{NSBezierPath, NSColor, NSFont};
use objc2_foundation::{NSPoint, NSRect, NSSize};

use qingjian_core::{GRID_ROWS, PANEL_COLUMNS};

use super::CandidateView;
use crate::candidates::PanelMouse;
use crate::candidates::frame::Frame;
use crate::candidates::panel_tabs::PanelTabs;

/// 面板尺寸固定，换组换分类窗口不跳：一行 [`PANEL_COLUMNS`] 个方格、[`GRID_ROWS`] 行，内容少的分类下面留空。
/// 颜文字这类宽条目少放几格（列数由 Core 按内容定），格子拉宽、总宽不变。
const CELL: f64 = 46.0;

/// 方格里的符号与 emoji 用大字；格子拉宽的（颜文字）用正常字号，才放得下。
const ITEM_POINTS: f64 = 24.0;
const WIDE_ITEM_POINTS: f64 = 17.0;

/// 分组胶囊、分类标签与信息行的字号。
const LABEL_POINTS: f64 = 13.0;

/// 格子之间的间距。
const CELL_GAP: f64 = 2.0;

/// 分组胶囊的左右、上下内边距与胶囊之间的间距。
const PILL_PAD_X: f64 = 9.0;
const PILL_PAD_Y: f64 = 2.5;
const PILL_GAP: f64 = 4.0;

/// 分类标签之间的间距与当前分类下划线的粗细。
const TAB_GAP: f64 = 14.0;
const UNDERLINE: f64 = 2.0;

/// 标签区与格子之间、格子与信息行之间的留白。
const SECTION_GAP: f64 = 8.0;

const HINT: &str = "Tab 换组 · [ ] 换类 · 双击或空格上屏";

/// 右上角关闭按钮（✕）的边长与它和按键提示的间距。
const CLOSE_SIZE: f64 = 18.0;
const CLOSE_GAP: f64 = 8.0;

/// 一帧面板里各块的位置（视图坐标，左上角为原点）。
struct PanelLayout {
    item_font: Retained<NSFont>,

    /// 分组胶囊。
    pills: Vec<NSRect>,

    /// 右上角的关闭按钮。
    close: NSRect,

    /// 看得见的分类标签：下标与位置。
    tabs: Vec<(usize, NSRect)>,

    /// 左右有没有藏起来的分类（画「‹」「›」）。
    more_left: bool,

    more_right: bool,

    /// 标签区下面的分隔线。
    divider: NSRect,

    /// 格子区左上角、一格宽高、每行几格。
    grid_origin: NSPoint,

    cell: NSSize,

    columns: usize,

    /// 信息行顶边。
    info_top: f64,

    left: f64,

    right: f64,
}

impl PanelLayout {
    fn cell_rect(&self, offset: usize) -> NSRect {
        let column = offset % self.columns;
        let row = offset / self.columns;
        NSRect::new(
            NSPoint::new(
                self.grid_origin.x + (self.cell.width + CELL_GAP) * column as f64,
                self.grid_origin.y + self.cell.height * row as f64,
            ),
            self.cell,
        )
    }
}

fn contains(rect: NSRect, point: NSPoint) -> bool {
    point.x >= rect.origin.x
        && point.x < rect.origin.x + rect.size.width
        && point.y >= rect.origin.y
        && point.y < rect.origin.y + rect.size.height
}

impl CandidateView {
    /// 面板一行格子多高，触控板滚动按它折算行数。
    pub(super) fn panel_row_height(&self) -> f64 {
        CELL
    }

    fn label_font(&self) -> Retained<NSFont> {
        NSFont::systemFontOfSize(LABEL_POINTS)
    }

    fn label_height(&self) -> f64 {
        self.measure("国", &self.label_font()).height
    }

    fn grid_width() -> f64 {
        CELL * PANEL_COLUMNS as f64 + CELL_GAP * (PANEL_COLUMNS - 1) as f64
    }

    /// 这一帧的格子：总宽固定，按列数均分；列数少的（颜文字）用正常字号。
    fn panel_cell(&self, frame: &Frame) -> (Retained<NSFont>, NSSize) {
        let columns = frame.columns.clamp(1, PANEL_COLUMNS);
        let width = (Self::grid_width() - CELL_GAP * (columns - 1) as f64) / columns as f64;
        let points = if columns >= PANEL_COLUMNS {
            ITEM_POINTS
        } else {
            WIDE_ITEM_POINTS
        };
        (NSFont::systemFontOfSize(points), NSSize::new(width, CELL))
    }

    /// 面板的宽高（不含窗口内边距），与内容无关。
    pub(super) fn panel_size(&self, _frame: &Frame) -> (f64, f64) {
        let small = self.label_height();
        let header_height = small + PILL_PAD_Y * 2.0 + SECTION_GAP / 2.0 + small + UNDERLINE + 3.0;
        (
            Self::grid_width(),
            header_height + 1.0 + SECTION_GAP + CELL * GRID_ROWS as f64 + SECTION_GAP / 2.0 + small,
        )
    }

    fn panel_layout(&self, frame: &Frame, tabs: &PanelTabs) -> PanelLayout {
        let theme = self.theme();
        let bounds = self.bounds();
        let (item_font, cell) = self.panel_cell(frame);
        let font = &self.label_font();
        let small = self.label_height();
        let left = theme.padding;
        let right = bounds.size.width - theme.padding;
        let mut y = theme.padding;

        let pill_height = small + PILL_PAD_Y * 2.0;
        let mut x = left;
        let pills = tabs
            .groups
            .iter()
            .map(|group| {
                let width = self.measure(group, font).width + PILL_PAD_X * 2.0;
                let rect = NSRect::new(NSPoint::new(x, y), NSSize::new(width, pill_height));
                x += width + PILL_GAP;
                rect
            })
            .collect();
        let close = NSRect::new(
            NSPoint::new(right - CLOSE_SIZE, y + (pill_height - CLOSE_SIZE) / 2.0),
            NSSize::new(CLOSE_SIZE, CLOSE_SIZE),
        );
        y += pill_height + SECTION_GAP / 2.0;

        // 分类标签放不下时以当前分类为中心往两边扩，两头藏起来的画「‹」「›」
        let widths: Vec<f64> = tabs
            .sections
            .iter()
            .map(|s| self.measure(s, font).width)
            .collect();
        let marker = self.measure("‹", font).width + TAB_GAP / 2.0;
        let available = right - left - marker * 2.0;
        let current = tabs.section.min(widths.len().saturating_sub(1));
        let (mut first, mut last) = (current, current);
        let mut used = widths.get(current).copied().unwrap_or(0.0);
        loop {
            let mut grew = false;
            if last + 1 < widths.len() && used + TAB_GAP + widths[last + 1] <= available {
                last += 1;
                used += TAB_GAP + widths[last];
                grew = true;
            }
            if first > 0 && used + TAB_GAP + widths[first - 1] <= available {
                first -= 1;
                used += TAB_GAP + widths[first];
                grew = true;
            }
            if !grew {
                break;
            }
        }
        let more_left = first > 0;
        let more_right = last + 1 < widths.len();
        let mut x = left + if more_left || more_right { marker } else { 0.0 };
        let tab_height = small + UNDERLINE + 3.0;
        let tabs_rects = (first..=last)
            .filter(|&i| i < widths.len())
            .map(|i| {
                let rect = NSRect::new(NSPoint::new(x, y), NSSize::new(widths[i], tab_height));
                x += widths[i] + TAB_GAP;
                (i, rect)
            })
            .collect();
        y += tab_height;
        let divider = NSRect::new(NSPoint::new(left, y), NSSize::new(right - left, 1.0));
        y += 1.0 + SECTION_GAP;

        let columns = frame.columns.clamp(1, PANEL_COLUMNS);
        let grid_origin = NSPoint::new(
            left + ((right - left) - Self::grid_width()).max(0.0) / 2.0,
            y,
        );
        PanelLayout {
            item_font,
            pills,
            close,
            tabs: tabs_rects,
            more_left,
            more_right,
            divider,
            grid_origin,
            cell,
            columns,
            info_top: y + CELL * GRID_ROWS as f64 + SECTION_GAP / 2.0,
            left,
            right,
        }
    }

    /// 点中的是哪一格、哪个胶囊或标签；空白处返回 `None`。
    pub(super) fn panel_hit(&self, point: NSPoint, double: bool) -> Option<PanelMouse> {
        let frame = self.ivars().frame.borrow();
        let tabs = frame.panel.as_ref()?;
        let layout = self.panel_layout(&frame, tabs);
        if contains(layout.close, point) {
            return Some(PanelMouse::Close);
        }
        if let Some(group) = layout.pills.iter().position(|r| contains(*r, point)) {
            return Some(PanelMouse::Group(group));
        }
        if let Some((section, _)) = layout.tabs.iter().find(|(_, r)| contains(*r, point)) {
            return Some(PanelMouse::Section(*section));
        }
        (0..frame.rows.len())
            .find(|&offset| {
                !frame.rows[offset].text.is_empty() && contains(layout.cell_rect(offset), point)
            })
            .map(|offset| PanelMouse::Cell { offset, double })
    }

    pub(super) fn draw_panel(&self, frame: &Frame, bounds: NSRect) {
        let Some(tabs) = frame.panel.as_ref() else {
            return;
        };
        let theme = self.theme();
        let font = &self.label_font();
        let layout = self.panel_layout(frame, tabs);
        let accent = NSColor::controlAccentColor();

        // 分组胶囊：当前组实心强调色、白字，其余淡字；右边是按键提示
        for (i, (group, rect)) in tabs.groups.iter().zip(&layout.pills).enumerate() {
            let active = i == tabs.group;
            if active {
                accent.set();
                NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(
                    *rect,
                    rect.size.height / 2.0,
                    rect.size.height / 2.0,
                )
                .fill();
            }
            let color = if active {
                NSColor::whiteColor()
            } else {
                theme.gloss_color.clone()
            };
            self.draw_text(
                group,
                font,
                &color,
                rect.origin.y + PILL_PAD_Y,
                rect.origin.x + PILL_PAD_X,
            );
        }
        if let Some(first) = layout.pills.first() {
            let hint = self.measure(HINT, &theme.index_font);
            self.draw_text(
                HINT,
                &theme.index_font,
                &theme.index_color,
                first.origin.y + (first.size.height - hint.height) / 2.0,
                (layout.close.origin.x - CLOSE_GAP - hint.width).max(bounds.origin.x),
            );
        }
        // 关闭按钮：淡色圆底上一个 ✕
        NSColor::quaternaryLabelColor().set();
        NSBezierPath::bezierPathWithOvalInRect(layout.close).fill();
        let cross = self.measure("✕", &theme.index_font);
        self.draw_text(
            "✕",
            &theme.index_font,
            &theme.gloss_color,
            layout.close.origin.y + (CLOSE_SIZE - cross.height) / 2.0,
            layout.close.origin.x + (CLOSE_SIZE - cross.width) / 2.0,
        );

        // 分类标签：当前分类正文色加强调色下划线
        let small = self.label_height();
        for (i, rect) in &layout.tabs {
            let active = *i == tabs.section;
            let color = if active {
                &theme.text_color
            } else {
                &theme.gloss_color
            };
            self.draw_text(
                &tabs.sections[*i],
                font,
                color,
                rect.origin.y,
                rect.origin.x,
            );
            if active {
                accent.set();
                NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(
                    NSRect::new(
                        NSPoint::new(rect.origin.x, rect.origin.y + small + 1.0),
                        NSSize::new(rect.size.width, UNDERLINE),
                    ),
                    UNDERLINE / 2.0,
                    UNDERLINE / 2.0,
                )
                .fill();
            }
        }
        if let Some((_, first)) = layout.tabs.first()
            && layout.more_left
        {
            self.draw_text("‹", font, &theme.index_color, first.origin.y, layout.left);
        }
        if let Some((_, last)) = layout.tabs.last()
            && layout.more_right
        {
            let width = self.measure("›", font).width;
            self.draw_text(
                "›",
                font,
                &theme.index_color,
                last.origin.y,
                layout.right - width,
            );
        }
        NSColor::separatorColor().set();
        NSBezierPath::fillRect(layout.divider);

        // 格子：条目居中，放不下的截断（信息行里看全文）
        for (offset, row) in frame.rows.iter().enumerate() {
            if row.text.is_empty() {
                continue;
            }
            let cell = layout.cell_rect(offset);
            if offset == frame.highlighted {
                self.fill_highlight(cell);
            }
            let (text, _) = self.truncate(&row.text, &layout.item_font, cell.size.width - 4.0);
            let size = self.measure(&text, &layout.item_font);
            self.draw_text(
                &text,
                &layout.item_font,
                &theme.text_color,
                cell.origin.y + (cell.size.height - size.height) / 2.0,
                cell.origin.x + (cell.size.width - size.width) / 2.0,
            );
        }

        // 信息行：左边是高亮条目与它的名字，右边是第几行
        let mut budget = layout.right - layout.left;
        if let Some(footer) = frame.footer.as_deref() {
            let width = self.measure(footer, &theme.index_font).width;
            self.draw_text(
                footer,
                &theme.index_font,
                &theme.index_color,
                layout.info_top,
                layout.right - width,
            );
            budget -= width + theme.column_gap;
        }
        if let Some(row) = frame.rows.get(frame.highlighted) {
            let mut x = layout.left;
            let used = self.draw_label(&row.text, &theme.text_color, x, layout.info_top, budget);
            x += used + theme.column_gap;
            budget -= used + theme.column_gap;
            for (segment, _) in &row.annotation {
                if budget <= 0.0 {
                    break;
                }
                let used = self.draw_label(segment, &theme.gloss_color, x, layout.info_top, budget);
                x += used;
                budget -= used;
            }
        }
    }

    /// 信息行里画一段标签字号的字，超过 `budget` 就截断；返回画了多宽。
    fn draw_label(&self, text: &str, color: &NSColor, x: f64, top: f64, budget: f64) -> f64 {
        let font = self.label_font();
        let (shown, _) = self.truncate(text, &font, budget);
        self.draw_text(&shown, &font, color, top, x)
    }
}
