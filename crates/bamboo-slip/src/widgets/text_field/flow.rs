//! 输入框里正文的排法，不依赖 [`crate::Ui`]：[`super::TextField`] 用它，自己摆位置的使用方（符号面板的翻译框）也能用。
//!
//! 正文按 [`tokens`] 切成一个个单位横排（多行时一排放不下折到下一排），光标是单位之间的一条竖线，组字接在光标前、带下划线，
//! 选区铺底。点击靠 [`FlowTarget`]：点中哪个单位、点在它的哪一侧，光标就落到哪；点在行尾后面的空白落到行尾。
//! 使用方布局之后按这些节点的位置接点击（`Ui` 用 `interact`，自己摆的用 [`crate::Scene::rect_in`]）。

use std::ops::Range;

use taffy::{AlignItems, FlexDirection, FlexWrap, LengthPercentageAuto, NodeId, Size, Style};

use super::tokens;
use crate::color::Color;
use crate::error::RenderError;
use crate::scene::{Scene, Visual};
use crate::text::{Decoration, TextStyle};

/// 密文显示成的圆点。
const BULLET: char = '•';

/// 一段正文怎么排、怎么画。位置都是 `text` 里的字节下标。
pub struct TextFlow<'a> {
    pub text: &'a str,

    /// 输入法正在组的字，接在光标处、带下划线。
    pub preedit: &'a str,

    pub caret: usize,

    pub selection: Range<usize>,

    /// 正文与组字都空时显示。
    pub placeholder: &'a str,

    /// 按换行分行、一排放不下就折行；单行时一排到底。
    pub multiline: bool,

    /// 有焦点才画光标与选区。
    pub focused: bool,

    /// 正文显示成圆点。
    pub secret: bool,

    pub style: TextStyle,

    pub placeholder_style: TextStyle,

    /// 光标竖线：宽、高（像素）与颜色。
    pub caret_bar: (f32, f32, Color),

    /// 选区的底。
    pub selection_paint: Visual,
}

/// 能点的节点，按建的先后排（`Ui` 的命中顺序跟它走）。
pub enum FlowTarget {
    /// 光标那条竖线。
    Caret(NodeId),

    /// 一个单位：点在它里面横向的几分之几，光标落到 `bounds` 里最近的那个字符边界。
    Token { node: NodeId, bounds: Vec<usize> },

    /// 一整行：点在行尾后面的空白，光标落到 `end`；`start` 是这一行开头，当标识用。
    LineEnd {
        node: NodeId,
        start: usize,
        end: usize,
    },
}

/// 建好的各行与能点的节点。
pub struct Flow {
    pub lines: Vec<NodeId>,

    pub caret: Option<NodeId>,

    pub targets: Vec<FlowTarget>,
}

impl FlowTarget {
    /// 点在这个单位横向 `fraction`（0 到 1）处，光标该落到哪。
    pub fn caret_at(bounds: &[usize], fraction: f32) -> usize {
        let last = bounds.len().saturating_sub(1);
        let index = (fraction.clamp(0.0, 1.0) * last as f32).round() as usize;
        bounds[index.min(last)]
    }
}

impl TextFlow<'_> {
    /// 每行一排：选区前、选区、选区后，或光标前、组字、光标、光标后。单行就是一排。
    pub fn build(&self, scene: &mut Scene) -> Result<Flow, RenderError> {
        let mut flow = Flow {
            lines: Vec::new(),
            caret: None,
            targets: Vec::new(),
        };
        if self.text.is_empty() && self.preedit.is_empty() {
            let mut pieces = Vec::new();
            if self.focused {
                let caret = self.caret(scene)?;
                pieces.push(caret);
                flow.caret = Some(caret);
                flow.targets.push(FlowTarget::Caret(caret));
            }
            if !self.placeholder.is_empty() {
                pieces.push(self.segment(scene, self.placeholder, &self.placeholder_style)?);
            }
            flow.lines.push(self.line(scene, &pieces)?);
            return Ok(flow);
        }
        let selection = &self.selection;
        let caret = self.caret;
        let mut start = 0;
        for line in self.text.split('\n') {
            let end = start + line.len();
            let mut pieces = Vec::new();
            if !self.focused {
                self.pieces(scene, start..end, &mut pieces, &mut flow.targets)?;
            } else if !selection.is_empty() {
                let (from, to) = (
                    selection.start.clamp(start, end),
                    selection.end.clamp(start, end),
                );
                self.pieces(scene, start..from, &mut pieces, &mut flow.targets)?;
                let before = pieces.len();
                self.pieces(scene, from..to, &mut pieces, &mut flow.targets)?;
                for piece in &mut pieces[before..] {
                    *piece =
                        scene.node(Style::default(), self.selection_paint.clone(), &[*piece])?;
                }
                self.pieces(scene, to..end, &mut pieces, &mut flow.targets)?;
            } else if (start..=end).contains(&caret) {
                self.pieces(scene, start..caret, &mut pieces, &mut flow.targets)?;
                let underlined = self.style.decorated(Decoration::Underline);
                for token in tokens(self.preedit) {
                    pieces.push(self.segment(scene, &self.preedit[token], &underlined)?);
                }
                let node = self.caret(scene)?;
                flow.caret = Some(node);
                flow.targets.push(FlowTarget::Caret(node));
                pieces.push(node);
                self.pieces(scene, caret..end, &mut pieces, &mut flow.targets)?;
            } else {
                self.pieces(scene, start..end, &mut pieces, &mut flow.targets)?;
            }
            let node = self.line(scene, &pieces)?;
            flow.targets.push(FlowTarget::LineEnd { node, start, end });
            flow.lines.push(node);
            start = end + 1;
        }
        Ok(flow)
    }

    /// `range` 那段正文按 [`tokens`] 切开，每个单位一个节点；单行的空串也建一个（各段的位置才稳定）。
    /// 单位里按字符数均分点击位置，汉字一个字一个单位所以是准的。
    fn pieces(
        &self,
        scene: &mut Scene,
        range: Range<usize>,
        out: &mut Vec<NodeId>,
        targets: &mut Vec<FlowTarget>,
    ) -> Result<(), RenderError> {
        let text = &self.text[range.clone()];
        if text.is_empty() && !self.multiline {
            out.push(self.segment(scene, "", &self.style)?);
            return Ok(());
        }
        for token in tokens(text) {
            let piece = &text[token.clone()];
            let shown = if self.secret {
                std::iter::repeat_n(BULLET, piece.chars().count()).collect()
            } else {
                piece.to_owned()
            };
            let node = self.segment(scene, &shown, &self.style)?;
            let at = range.start + token.start;
            let bounds: Vec<usize> = piece
                .char_indices()
                .map(|(i, _)| at + i)
                .chain(std::iter::once(range.start + token.end))
                .collect();
            targets.push(FlowTarget::Token { node, bounds });
            out.push(node);
        }
        Ok(())
    }

    /// 一行：各段横排，空行也有一行高；多行时放不下的折到下一排。
    fn line(&self, scene: &mut Scene, pieces: &[NodeId]) -> Result<NodeId, RenderError> {
        scene.node(
            Style {
                flex_direction: FlexDirection::Row,
                flex_shrink: 0.0,
                flex_wrap: if self.multiline {
                    FlexWrap::Wrap
                } else {
                    FlexWrap::NoWrap
                },
                align_items: AlignItems::CENTER,
                min_size: Size {
                    width: LengthPercentageAuto::auto(),
                    height: LengthPercentageAuto::length(self.style.line_height),
                },
                ..Style::default()
            },
            Visual::Group,
            pieces,
        )
    }

    fn caret(&self, scene: &mut Scene) -> Result<NodeId, RenderError> {
        let (width, height, color) = self.caret_bar;
        scene.node(
            Style {
                size: Size {
                    width: taffy::Dimension::length(width),
                    height: taffy::Dimension::length(height),
                },
                flex_shrink: 0.0,
                ..Style::default()
            },
            Visual::solid(color, 0.0),
            &[],
        )
    }

    /// 一段文字；空串也建（占位为零宽），各段的位置才稳定。
    fn segment(
        &self,
        scene: &mut Scene,
        text: &str,
        style: &TextStyle,
    ) -> Result<NodeId, RenderError> {
        scene.node(
            Style {
                flex_shrink: 0.0,
                ..Style::default()
            },
            Visual::Text {
                text: text.to_owned(),
                style: *style,
            },
            &[],
        )
    }
}
