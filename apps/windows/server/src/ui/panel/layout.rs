//! 面板各块的位置：顶上分组胶囊与分类标签，中间等宽格子，底下信息行。尺寸固定，换组换分类窗口不跳；
//! 画与点击共用这一份。数值与 macOS 端的系统绘制一致（逻辑像素，按 DPI 缩放）。

use windows::Win32::Graphics::DirectWrite::IDWriteTextFormat;
use windows::core::Result;

use qingjian_core::{GRID_ROWS, PANEL_COLUMNS};

use super::d2d::{Direct2D, Rect};
use crate::dispatch::{PanelFrame, PanelMouse};

const PADDING: f32 = 10.0;
const CELL: f32 = 46.0;
const CELL_GAP: f32 = 2.0;

/// 方格里的符号与 emoji 用大字；格子拉宽的（颜文字）用正常字号，才放得下。
const ITEM_PX: f32 = 24.0;
const WIDE_ITEM_PX: f32 = 17.0;

/// 胶囊、标签、信息行的字号与按键提示的字号。
const LABEL_PX: f32 = 14.0;
const HINT_PX: f32 = 12.0;

const PILL_PAD_X: f32 = 10.0;
const PILL_PAD_Y: f32 = 3.0;
const PILL_GAP: f32 = 4.0;

const TAB_GAP: f32 = 14.0;
const UNDERLINE: f32 = 2.0;

/// 标签区与格子之间、格子与信息行之间的留白。
const SECTION_GAP: f32 = 8.0;

/// 右上角关闭按钮的直径与它和按键提示的间距。
const CLOSE_SIZE: f32 = 20.0;
const CLOSE_GAP: f32 = 8.0;

pub(super) const HINT: &str = "Tab 换组 · [ ] 换类 · 双击或空格上屏";

/// 按 DPI 建好的字体。
pub(super) struct Fonts {
    pub dpi: u32,

    pub label: IDWriteTextFormat,

    pub hint: IDWriteTextFormat,

    pub item: IDWriteTextFormat,

    pub wide_item: IDWriteTextFormat,

    /// 标签字号一行多高。
    pub label_height: f32,
}

impl Fonts {
    pub(super) fn new(d2d: &Direct2D, dpi: u32) -> Result<Self> {
        let scale = dpi as f32 / 96.0;
        let label = d2d.format(LABEL_PX * scale)?;
        let label_height = d2d.measure("国", &label).1;
        Ok(Self {
            dpi,
            hint: d2d.format(HINT_PX * scale)?,
            item: d2d.format(ITEM_PX * scale)?,
            wide_item: d2d.format(WIDE_ITEM_PX * scale)?,
            label,
            label_height,
        })
    }
}

pub(super) struct PanelLayout {
    /// 内容区宽高（物理像素）。
    pub size: (i32, i32),

    pub scale: f32,

    pub pills: Vec<Rect>,

    pub close: Rect,

    /// 看得见的分类标签：下标与位置。
    pub tabs: Vec<(usize, Rect)>,

    /// 左右有没有藏起来的分类（画「‹」「›」）。
    pub more_left: bool,

    pub more_right: bool,

    pub divider: Rect,

    pub grid: (f32, f32),

    pub cell: (f32, f32),

    pub columns: usize,

    /// 每行几格时用大字（十格一行）。
    pub narrow: bool,

    pub info_top: f32,

    pub left: f32,

    pub right: f32,
}

impl PanelLayout {
    pub(super) fn new(d2d: &Direct2D, fonts: &Fonts, frame: &PanelFrame) -> Self {
        let scale = fonts.dpi as f32 / 96.0;
        let px = |v: f32| v * scale;
        let small = fonts.label_height;
        let grid_width =
            px(CELL) * PANEL_COLUMNS as f32 + px(CELL_GAP) * (PANEL_COLUMNS - 1) as f32;
        let left = px(PADDING);
        let right = left + grid_width;
        let mut y = px(PADDING);

        let pill_height = small + px(PILL_PAD_Y) * 2.0;
        let mut x = left;
        let pills = frame
            .groups
            .iter()
            .map(|group| {
                let width = d2d.measure(group, &fonts.label).0 + px(PILL_PAD_X) * 2.0;
                let rect = Rect::new(x, y, width, pill_height);
                x += width + px(PILL_GAP);
                rect
            })
            .collect();
        let close = Rect::new(
            right - px(CLOSE_SIZE),
            y + (pill_height - px(CLOSE_SIZE)) / 2.0,
            px(CLOSE_SIZE),
            px(CLOSE_SIZE),
        );
        y += pill_height + px(SECTION_GAP) / 2.0;

        // 分类标签放不下时以当前分类为中心往两边扩，两头藏起来的画「‹」「›」
        let widths: Vec<f32> = frame
            .sections
            .iter()
            .map(|s| d2d.measure(s, &fonts.label).0)
            .collect();
        let gap = px(TAB_GAP);
        let marker = d2d.measure("‹", &fonts.label).0 + gap / 2.0;
        let available = right - left - marker * 2.0;
        let current = frame.section.min(widths.len().saturating_sub(1));
        let (mut first, mut last) = (current, current);
        let mut used = widths.get(current).copied().unwrap_or(0.0);
        loop {
            let mut grew = false;
            if last + 1 < widths.len() && used + gap + widths[last + 1] <= available {
                last += 1;
                used += gap + widths[last];
                grew = true;
            }
            if first > 0 && used + gap + widths[first - 1] <= available {
                first -= 1;
                used += gap + widths[first];
                grew = true;
            }
            if !grew {
                break;
            }
        }
        let more_left = first > 0;
        let more_right = last + 1 < widths.len();
        let mut x = left + if more_left || more_right { marker } else { 0.0 };
        let tab_height = small + px(UNDERLINE) + px(3.0);
        let tabs = (first..=last)
            .filter(|&i| i < widths.len())
            .map(|i| {
                let rect = Rect::new(x, y, widths[i], tab_height);
                x += widths[i] + gap;
                (i, rect)
            })
            .collect();
        y += tab_height;
        let divider = Rect::new(left, y, right - left, px(1.0).max(1.0));
        y += px(1.0) + px(SECTION_GAP);

        let columns = frame.columns.clamp(1, PANEL_COLUMNS);
        let cell_width = (grid_width - px(CELL_GAP) * (columns - 1) as f32) / columns as f32;
        let grid = (left, y);
        let info_top = y + px(CELL) * GRID_ROWS as f32 + px(SECTION_GAP) / 2.0;
        let height = info_top + small + px(PADDING);
        Self {
            size: ((right + px(PADDING)).ceil() as i32, height.ceil() as i32),
            scale,
            pills,
            close,
            tabs,
            more_left,
            more_right,
            divider,
            grid,
            cell: (cell_width, px(CELL)),
            columns,
            narrow: columns >= PANEL_COLUMNS,
            info_top,
            left,
            right,
        }
    }

    pub(super) fn cell_rect(&self, offset: usize) -> Rect {
        let column = (offset % self.columns) as f32;
        let row = (offset / self.columns) as f32;
        Rect::new(
            self.grid.0 + (self.cell.0 + CELL_GAP * self.scale) * column,
            self.grid.1 + self.cell.1 * row,
            self.cell.0,
            self.cell.1,
        )
    }

    pub(super) fn underline(&self) -> f32 {
        UNDERLINE * self.scale
    }

    pub(super) fn close_gap(&self) -> f32 {
        CLOSE_GAP * self.scale
    }

    pub(super) fn pill_pad(&self) -> (f32, f32) {
        (PILL_PAD_X * self.scale, PILL_PAD_Y * self.scale)
    }

    /// 内容坐标 `(x, y)` 点中了什么；空白处返回 `None`。
    pub(super) fn hit(
        &self,
        frame: &PanelFrame,
        x: f32,
        y: f32,
        double: bool,
    ) -> Option<PanelMouse> {
        if self.close.contains(x, y) {
            return Some(PanelMouse::Close);
        }
        if let Some(group) = self.pills.iter().position(|r| r.contains(x, y)) {
            return Some(PanelMouse::Group(group));
        }
        if let Some((section, _)) = self.tabs.iter().find(|(_, r)| r.contains(x, y)) {
            return Some(PanelMouse::Section(*section));
        }
        (0..frame.cells.len())
            .find(|&offset| frame.cells[offset].is_some() && self.cell_rect(offset).contains(x, y))
            .map(|offset| PanelMouse::Cell { offset, double })
    }

    /// 内容区里收不收鼠标：圆角内都收，滚轮在空白处也能滚。
    pub(super) fn inside(&self, x: f32, y: f32) -> bool {
        x >= 0.0 && y >= 0.0 && x < self.size.0 as f32 && y < self.size.1 as f32
    }
}
