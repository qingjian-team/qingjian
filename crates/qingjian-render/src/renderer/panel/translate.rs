//! 翻译组的格子区：上面是原文编辑区（bamboo-slip 的 [`TextFlow`] 排：折行、光标、还在打的拼音带下划线嵌在光标处），
//! 选中的字铺高亮底色，放不下时往上滚到能看见光标；下面一行是拼音的候选（高亮候选铺底），信息行写目标语言与翻译进度。
//! 占的高度与格子区一样，换组窗口不跳。原文里点哪里光标放到哪里、拖到哪里选到哪里（[`PanelHit::Caret`]），要布局之后才知道位置，见 [`TextBox`]。

use bamboo_slip::taffy::{FlexDirection, LengthPercentageAuto, NodeId, Rect, Style};
use bamboo_slip::{FlowTarget, RenderError, Scene, TextFlow, TextPainter, TextStyle, Visual};

use super::{Area, Canvas, Metrics, placed_with};
use crate::panel::PanelTranslate;
use crate::renderer::{HitRegion, HitTarget, PanelHit};

/// 候选之间、序号与候选之间的间距（点）。
const CANDIDATE_GAP: f32 = 6.0;

/// 候选文字两边留白（点）。
const CANDIDATE_PADDING: f32 = 8.0;

/// 原文区与候选行之间的间距（点）。
const SOURCE_GAP: f32 = 6.0;

/// 光标竖线的宽（点）。
const CARET_WIDTH: f32 = 1.5;

/// 选区底色的圆角（点）。
const SELECTION_RADIUS: f32 = 2.0;

/// 画翻译框要用的几种字。
pub(super) struct Styles {
    /// 正文色的标签字：提示行。
    pub(super) text: TextStyle,

    /// 次要色的标签字：候选序号。
    pub(super) label: TextStyle,

    /// 提示色的小字：空框提示。
    pub(super) hint: TextStyle,
}

/// 原文编辑区布局之后要补的事：滚到光标、按各个字的位置给出点击区域。
pub(super) struct TextBox {
    /// 裁剪用的视口。
    view: NodeId,

    /// 视口里的各行，滚动时整体上移。
    content: NodeId,

    caret: Option<NodeId>,

    targets: Vec<FlowTarget>,

    /// 原文：点击给的字节下标按它换算成第几个字之后。
    source: String,
}

pub(super) fn draw(
    canvas: &mut Canvas,
    m: &Metrics,
    text: &mut TextPainter,
    frame: &PanelTranslate,
    styles: &Styles,
    area: Area,
    info_top: f32,
) -> Result<TextBox, RenderError> {
    let spec = &m.theme.file().panel;
    let item = m.style(m.theme.font_ref(&spec.wide_item_font), &spec.text);
    let row_height = m.px(spec.cell);
    let row_top = area.y + area.height - row_height;

    // 原文
    let source_area = Area::new(
        area.x,
        area.y,
        area.width,
        (row_top - m.px(SOURCE_GAP) - area.y).max(0.0),
    );
    let byte_at = |chars: usize| {
        frame
            .source
            .char_indices()
            .nth(chars)
            .map_or(frame.source.len(), |(i, _)| i)
    };
    let caret = byte_at(frame.cursor);
    let selection = match &frame.selection {
        Some(range) if frame.composing.is_empty() => byte_at(range.start)..byte_at(range.end),
        _ => caret..caret,
    };
    let flow = TextFlow {
        text: &frame.source,
        preedit: &frame.composing,
        caret,
        selection,
        placeholder: &frame.placeholder,
        multiline: true,
        focused: true,
        secret: false,
        style: item,
        placeholder_style: styles.hint,
        caret_bar: (m.px(CARET_WIDTH), item.line_height, item.color),
        selection_paint: m.fill(&spec.highlight, m.px(SELECTION_RADIUS)),
    }
    .build(&mut canvas.scene)?;
    let content = canvas.scene.node(
        Style {
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            size: bamboo_slip::taffy::Size {
                width: bamboo_slip::taffy::Dimension::length(source_area.width),
                height: bamboo_slip::taffy::Dimension::auto(),
            },
            ..Style::default()
        },
        Visual::Group,
        &flow.lines,
    )?;
    let view = placed_with(&mut canvas.scene, source_area, Visual::Group, &[content])?;
    canvas.scene.set_clip(view, 0.0);
    canvas.nodes.push(view);

    // 候选
    let padding = m.px(CANDIDATE_PADDING);
    let gap = m.px(CANDIDATE_GAP);
    let radius = m.px(spec.cell_radius);
    let label_line = text.measure("国", &styles.label).height;
    let index_width = text.measure("9", &styles.label).width;
    let mut x = area.x;
    for (i, candidate) in frame.candidates.iter().enumerate() {
        let word = text.measure(candidate, &item);
        let width = padding * 2.0 + index_width + gap / 2.0 + word.width;
        if x + width > area.x + area.width {
            break;
        }
        let cell = Area::new(x, row_top, width, row_height);
        if i == frame.highlight {
            canvas.add(cell, m.fill(&spec.highlight, radius))?;
        }
        let number = (i + 1).to_string();
        let number_top = row_top + (row_height - label_line) / 2.0;
        canvas.text(
            &number,
            &styles.label,
            (x + padding, number_top),
            (index_width + 1.0, label_line),
        )?;
        let at = (
            x + padding + index_width + gap / 2.0,
            row_top + (row_height - word.height) / 2.0,
        );
        canvas.text(candidate, &item, at, (word.width + 1.0, word.height))?;
        canvas
            .hits
            .push(cell.hit(HitTarget::Panel(PanelHit::Candidate(i))));
        x += width + gap;
    }

    // 信息行：目标语言与进度
    let width = text
        .measure(&frame.status, &styles.text)
        .width
        .min(area.width);
    canvas.text(
        &frame.status,
        &styles.text,
        (area.x, info_top),
        (width + 1.0, label_line),
    )?;
    Ok(TextBox {
        view,
        content,
        caret: flow.caret,
        targets: flow.targets,
        source: frame.source.clone(),
    })
}

impl TextBox {
    /// 光标跑到视口下面了：内容上移到正好露出它。返回挪没挪（挪了要再布局一次）。
    pub(super) fn follow_caret(
        &self,
        scene: &mut Scene,
        root: NodeId,
    ) -> Result<bool, RenderError> {
        let Some(caret) = self.caret else {
            return Ok(false);
        };
        let (_, caret_top, _, caret_height) = scene.rect_in(caret, root)?;
        let (_, view_top, _, view_height) = scene.rect_in(self.view, root)?;
        let lift = (caret_top + caret_height - (view_top + view_height)).max(0.0);
        if lift <= 0.0 {
            return Ok(false);
        }
        scene.restyle(self.content, |style| {
            style.margin = Rect {
                top: LengthPercentageAuto::length(-lift),
                ..Rect::zero()
            };
        })?;
        Ok(true)
    }

    /// 原文里的点击区域：每个字左右两半各落到它前后的边界上，行尾后面的空白落到行尾，最后一行下面落到末尾；
    /// 只给视口里看得见的部分。
    pub(super) fn hits(&self, scene: &Scene, root: NodeId) -> Result<Vec<HitRegion>, RenderError> {
        let view = scene.rect_in(self.view, root)?;
        let clip = |(x, y, width, height): (f32, f32, f32, f32)| {
            let top = y.max(view.1);
            let bottom = (y + height).min(view.1 + view.3);
            (bottom > top).then_some((x, top, width, bottom - top))
        };
        let caret_at = |byte: usize| {
            let position = self.source[..byte.min(self.source.len())].chars().count();
            HitTarget::Panel(PanelHit::Caret(position))
        };
        let mut regions = Vec::new();
        let mut line_ends = Vec::new();
        for target in &self.targets {
            match target {
                FlowTarget::Token { node, bounds } => {
                    let Some((x, y, width, height)) = clip(scene.rect_in(*node, root)?) else {
                        continue;
                    };
                    let slices = bounds.len().saturating_sub(1).max(1) as f32;
                    let step = width / slices;
                    for (j, bound) in bounds.iter().enumerate() {
                        let left = (x + step * (j as f32 - 0.5)).max(x);
                        let right = (x + step * (j as f32 + 0.5)).min(x + width);
                        if right > left {
                            regions.push(HitRegion {
                                x: left,
                                y,
                                width: right - left,
                                height,
                                target: caret_at(*bound),
                            });
                        }
                    }
                }
                FlowTarget::LineEnd { node, end, .. } => {
                    if let Some((x, y, width, height)) = clip(scene.rect_in(*node, root)?) {
                        line_ends.push(HitRegion {
                            x,
                            y,
                            width,
                            height,
                            target: caret_at(*end),
                        });
                    }
                }
                FlowTarget::Caret(_) => {}
            }
        }
        regions.append(&mut line_ends);
        regions.push(HitRegion {
            x: view.0,
            y: view.1,
            width: view.2,
            height: view.3,
            target: caret_at(self.source.len()),
        });
        Ok(regions)
    }
}
