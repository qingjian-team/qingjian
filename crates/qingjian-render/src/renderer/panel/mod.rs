//! 符号面板：顶上分组胶囊（当前组实心）、按键提示与关闭按钮，下一行分类标签（当前分类加下划线，放不下时两头画「‹」「›」），
//! 中间等宽格子（条目居中、高亮格铺底），底下信息行（高亮条目、它的名字与行号）；翻译组的格子区换成文本区（见 [`translate`]）。
//! 排法固定、尺寸与内容无关（换组换分类窗口不跳），字体、尺寸与颜色取主题的 `panel` 一节；各块的矩形就是点击区域。

mod tabs;
mod translate;

use bamboo_slip::taffy::{Dimension, LengthPercentageAuto, NodeId, Position, Rect, Size, Style};
use bamboo_slip::{BoxPaint, Color, FontSpec, RenderError, Scene, TextStyle, Visual};

use super::build::resolve_fill;
use super::{HitRegion, HitTarget, PanelHit, Rendered, Renderer};
use crate::icon::Icon;
use crate::panel::PanelFrame;
use crate::theme::file::node::FillSpec;
use crate::theme::file::{ColorRef, ColorSpec};
use crate::theme::{TextSizes, Theme};

/// 一行排满几格，与 Core 的 `PANEL_COLUMNS` 一致；格子区总宽按它算。
const FULL_COLUMNS: usize = 10;

/// 格里条目离格边至少留多少（点）。
const CELL_INSET: f32 = 2.0;

/// 按键提示与关闭按钮、信息行里各段之间的间距（点）。
const GAP: f32 = 8.0;

/// 下划线离标签文字底边多远（点）。
const UNDERLINE_OFFSET: f32 = 1.0;

/// 关闭按钮的点击区域往外放几点，免得难点中。
const CLOSE_SLOP: f32 = 4.0;

/// 一次渲染里按倍数换算好的参数。
struct Metrics<'a> {
    theme: &'a Theme,

    scale: f32,
}

impl Metrics<'_> {
    fn px(&self, points: f32) -> f32 {
        points * self.scale
    }

    fn style(&self, font: FontSpec, color: &ColorRef) -> TextStyle {
        TextStyle::new(
            font.scaled(self.scale),
            font.size,
            self.theme.color(color),
            self.theme.text_gamma(),
        )
    }

    /// 面板没有数据条件，条件颜色取 `else` 分支。
    fn color(&self, spec: &ColorSpec) -> Color {
        self.theme.color(fixed(spec))
    }

    fn fill(&self, fill: &FillSpec, radius: f32) -> Visual {
        Visual::Box(BoxPaint {
            fill: resolve_fill(self.theme, self.scale, fill, |spec| self.color(spec)),
            border: None,
            radius,
        })
    }
}

fn fixed(spec: &ColorSpec) -> &ColorRef {
    match spec {
        ColorSpec::Fixed(color)
        | ColorSpec::Switch {
            otherwise: color, ..
        } => color,
    }
}

/// 一块矩形（内容区像素）。
#[derive(Debug, Clone, Copy)]
struct Area {
    x: f32,

    y: f32,

    width: f32,

    height: f32,
}

impl Area {
    fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    fn hit(self, target: HitTarget) -> HitRegion {
        HitRegion {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
            target,
        }
    }
}

/// 绝对定位在 `area` 的节点。
fn placed(scene: &mut Scene, area: Area, visual: Visual) -> Result<NodeId, RenderError> {
    placed_with(scene, area, visual, &[])
}

/// 绝对定位在 `area`、带子节点的节点。
fn placed_with(
    scene: &mut Scene,
    area: Area,
    visual: Visual,
    children: &[NodeId],
) -> Result<NodeId, RenderError> {
    let style = Style {
        position: Position::Absolute,
        inset: Rect {
            left: LengthPercentageAuto::length(area.x),
            top: LengthPercentageAuto::length(area.y),
            right: LengthPercentageAuto::auto(),
            bottom: LengthPercentageAuto::auto(),
        },
        size: Size {
            width: Dimension::length(area.width),
            height: Dimension::length(area.height),
        },
        ..Style::default()
    };
    scene.node(style, visual, children)
}

/// 建场景时收集子节点与点击区域。
struct Canvas {
    scene: Scene,

    nodes: Vec<NodeId>,

    hits: Vec<HitRegion>,
}

impl Canvas {
    fn add(&mut self, area: Area, visual: Visual) -> Result<(), RenderError> {
        let node = placed(&mut self.scene, area, visual)?;
        self.nodes.push(node);
        Ok(())
    }

    /// 一行字，左上角在 `(x, y)`，宽 `width`（放不下截尾）。
    fn text(
        &mut self,
        text: &str,
        style: &TextStyle,
        (x, y): (f32, f32),
        size: (f32, f32),
    ) -> Result<(), RenderError> {
        let visual = Visual::Text {
            text: text.to_owned(),
            style: style.truncated(),
        };
        self.add(Area::new(x, y, size.0, size.1), visual)
    }
}

impl Renderer {
    /// 画符号面板。不跟设置里的候选字号走（面板尺寸固定），`scale` 是点 → 像素的倍数。
    pub fn render_panel(
        &mut self,
        frame: &PanelFrame,
        theme: &Theme,
        scale: f32,
    ) -> Result<Rendered, RenderError> {
        let theme = &theme.with_text_sizes(TextSizes::default());
        self.stage.text().use_families(theme.families());
        let spec = &theme.file().panel;
        let m = Metrics { theme, scale };
        let label_font = theme.font_ref(&spec.label_font);
        let hint_font = theme.font_ref(&spec.hint_font);
        let label = m.style(label_font, &spec.label);
        let hint = m.style(hint_font, &spec.hint);
        let text = self.stage.text();
        let line = text.measure("国", &label).height;
        let hint_line = text.measure("国", &hint).height;

        let padding = m.px(spec.padding);
        let cell = m.px(spec.cell);
        let cell_gap = m.px(spec.cell_gap);
        let grid_width = cell * FULL_COLUMNS as f32 + cell_gap * (FULL_COLUMNS - 1) as f32;
        let (left, right) = (padding, padding + grid_width);

        // 先排好竖直方向，内容区尺寸与内容无关
        let pill_height = line + m.px(spec.pill_padding_y) * 2.0;
        let pills_top = padding;
        let tabs_top = pills_top + pill_height + m.px(spec.section_gap) / 2.0;
        let tab_height = line + m.px(UNDERLINE_OFFSET + spec.underline + 2.0);
        let divider_top = tabs_top + tab_height;
        let divider_height = m.px(1.0).max(1.0);
        let grid_top = divider_top + divider_height + m.px(spec.section_gap);
        let rows = frame.rows.max(1);
        let info_top = grid_top + cell * rows as f32 + m.px(spec.section_gap) / 2.0;
        let content = ((right + padding).ceil(), (info_top + line + padding).ceil());

        let mut canvas = Canvas {
            scene: Scene::new(),
            nodes: Vec::new(),
            hits: Vec::new(),
        };

        // 分组胶囊：当前组实心
        let mut x = left;
        for (i, group) in frame.groups.iter().enumerate() {
            let active = i == frame.group;
            let width = text.measure(group, &label).width;
            let area = Area::new(
                x,
                pills_top,
                width + m.px(spec.pill_padding_x) * 2.0,
                pill_height,
            );
            if active {
                canvas.add(area, m.fill(&spec.pill_fill, pill_height / 2.0))?;
            }
            let style = if active {
                m.style(label_font, &spec.pill_text)
            } else {
                label
            };
            let at = (
                area.x + m.px(spec.pill_padding_x),
                area.y + m.px(spec.pill_padding_y),
            );
            canvas.text(group, &style, at, (width + 1.0, line))?;
            canvas
                .hits
                .push(area.hit(HitTarget::Panel(PanelHit::Group(i))));
            x = area.x + area.width + m.px(spec.pill_gap);
        }

        // 关闭按钮与它左边的按键提示（贴右，放不下截尾）
        let close_size = m.px(spec.close_size);
        let close = Area::new(
            right - close_size,
            pills_top + (pill_height - close_size) / 2.0,
            close_size,
            close_size,
        );
        canvas.add(
            close,
            Visual::Box(BoxPaint {
                fill: Some(bamboo_slip::Fill::Solid(theme.color(&spec.close_fill))),
                border: None,
                radius: close_size / 2.0,
            }),
        )?;
        canvas.add(
            close,
            Visual::Icon {
                icon: Icon::Close.painter(),
                size: close_size,
                color: theme.color(&spec.close_icon),
            },
        )?;
        let slop = m.px(CLOSE_SLOP);
        canvas.hits.push(
            Area::new(
                close.x - slop,
                close.y - slop,
                close.width + slop * 2.0,
                close.height + slop * 2.0,
            )
            .hit(HitTarget::Panel(PanelHit::Close)),
        );
        let hint_start = x - m.px(spec.pill_gap) + m.px(GAP);
        let hint_end = close.x - m.px(GAP);
        if hint_end > hint_start && !frame.hint.is_empty() {
            let width = text
                .measure(&frame.hint, &hint)
                .width
                .min(hint_end - hint_start);
            let top = pills_top + (pill_height - hint_line) / 2.0;
            canvas.text(
                &frame.hint,
                &hint,
                (hint_end - width, top),
                (width, hint_line),
            )?;
        }

        // 分类标签：当前分类正文色加下划线；放不下时以当前分类为中心，两头藏起来的画「‹」「›」
        let widths: Vec<f32> = frame
            .sections
            .iter()
            .map(|section| text.measure(section, &label).width)
            .collect();
        let tab_gap = m.px(spec.tab_gap);
        let marker_width = text.measure("‹", &label).width;
        let marker = marker_width + tab_gap / 2.0;
        let fits = widths.iter().sum::<f32>() + tab_gap * widths.len().saturating_sub(1) as f32
            <= right - left;
        let (first, last) = if fits {
            (0, widths.len().saturating_sub(1))
        } else {
            tabs::visible(&widths, frame.section, tab_gap, right - left - marker * 2.0)
        };
        let more_left = first > 0;
        let more_right = last + 1 < widths.len();
        let mut x = left + if fits { 0.0 } else { marker };
        let current = m.style(label_font, &spec.text);
        for i in (first..=last).filter(|&i| i < widths.len()) {
            let active = i == frame.section;
            let style = if active { &current } else { &label };
            canvas.text(
                &frame.sections[i],
                style,
                (x, tabs_top),
                (widths[i] + 1.0, line),
            )?;
            if active {
                let underline = Area::new(
                    x,
                    tabs_top + line + m.px(UNDERLINE_OFFSET),
                    widths[i],
                    m.px(spec.underline),
                );
                canvas.add(
                    underline,
                    Visual::solid(theme.color(&spec.underline_color), underline.height / 2.0),
                )?;
            }
            canvas.hits.push(
                Area::new(x, tabs_top, widths[i], tab_height)
                    .hit(HitTarget::Panel(PanelHit::Section(i))),
            );
            x += widths[i] + tab_gap;
        }
        let markers = m.style(label_font, &spec.hint);
        if more_left {
            canvas.text("‹", &markers, (left, tabs_top), (marker_width + 1.0, line))?;
        }
        if more_right {
            let at = (right - marker_width, tabs_top);
            canvas.text("›", &markers, at, (marker_width + 1.0, line))?;
        }
        canvas.add(
            Area::new(left, divider_top, right - left, divider_height),
            Visual::solid(theme.color(&spec.divider), 0.0),
        )?;

        let mut text_box = None;
        if let Some(translate) = &frame.translate {
            let area = Area::new(left, grid_top, right - left, cell * rows as f32);
            let styles = translate::Styles {
                text: current,
                label,
                hint,
            };
            text_box = Some(translate::draw(
                &mut canvas,
                &m,
                text,
                translate,
                &styles,
                area,
                info_top,
            )?);
        } else {
            // 格子：列数少的（颜文字）格子拉宽、用小一号的字，放不下的截尾（信息行里看全文）
            let columns = frame.columns.clamp(1, FULL_COLUMNS);
            let cell_width = (grid_width - cell_gap * (columns - 1) as f32) / columns as f32;
            let item_font = if columns >= FULL_COLUMNS {
                &spec.item_font
            } else {
                &spec.wide_item_font
            };
            let item = m.style(theme.font_ref(item_font), &spec.text);
            let inset = m.px(CELL_INSET);
            let cell_radius = m.px(spec.cell_radius);
            for (offset, entry) in frame.cells.iter().enumerate() {
                let Some(entry) = entry else {
                    continue;
                };
                let column = offset % columns;
                let row = offset / columns;
                if row >= rows {
                    break;
                }
                let area = Area::new(
                    left + (cell_width + cell_gap) * column as f32,
                    grid_top + cell * row as f32,
                    cell_width,
                    cell,
                );
                if offset == frame.highlight {
                    canvas.add(area, m.fill(&spec.highlight, cell_radius))?;
                }
                let size = text.measure(&entry.text, &item);
                let width = size.width.min(area.width - inset * 2.0);
                let at = (
                    area.x + (area.width - width) / 2.0,
                    area.y + (area.height - size.height) / 2.0,
                );
                canvas.text(&entry.text, &item, at, (width + 1.0, size.height))?;
                canvas
                    .hits
                    .push(area.hit(HitTarget::Panel(PanelHit::Cell(offset))));
            }

            // 信息行：左边是高亮条目与它的名字，右边是第几行
            let gap = m.px(GAP);
            let mut budget = right - left;
            if let Some(footer) = frame.footer.as_deref() {
                let width = text.measure(footer, &hint).width.min(budget);
                let top = info_top + (line - hint_line) / 2.0;
                canvas.text(
                    footer,
                    &hint,
                    (right - width, top),
                    (width + 1.0, hint_line),
                )?;
                budget -= width + gap;
            }
            if let Some(Some(entry)) = frame.cells.get(frame.highlight) {
                let entry_width = text
                    .measure(&entry.text, &current)
                    .width
                    .min(budget.max(0.0));
                let size = (entry_width + 1.0, line);
                canvas.text(&entry.text, &current, (left, info_top), size)?;
                budget -= entry_width + gap;
                if let Some(name) = entry.name.as_deref()
                    && budget > 0.0
                {
                    let width = text.measure(name, &label).width.min(budget);
                    let at = (left + entry_width + gap, info_top);
                    canvas.text(name, &label, at, (width + 1.0, line))?;
                }
            }
        }

        let Canvas {
            mut scene,
            nodes,
            mut hits,
        } = canvas;
        let root_style = Style {
            size: Size {
                width: Dimension::length(content.0),
                height: Dimension::length(content.1),
            },
            ..Style::default()
        };
        let root = scene.node(
            root_style,
            m.fill(&spec.background, m.px(spec.radius)),
            &nodes,
        )?;
        let effects = spec
            .effects
            .iter()
            .map(|effect| effect.to_effect(scale, m.color(&effect.shadow().color)))
            .collect();
        scene.set_effects(root, effects);
        scene.layout(root, self.stage.text())?;
        if let Some(text_box) = &text_box {
            if text_box.follow_caret(&mut scene, root)? {
                scene.layout(root, self.stage.text())?;
            }
            // 原文里的点击排在最前（各块不重叠，排前只为保险）
            let mut regions = text_box.hits(&scene, root)?;
            regions.append(&mut hits);
            hits = regions;
        }
        let painted = self.stage.rasterize(&scene, root, content, scale)?;
        Ok(Rendered::new(painted, hits))
    }
}

#[cfg(test)]
mod tests;
