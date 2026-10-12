//! 场景图：一棵 Taffy 布局树，每个节点带一个 [`Visual`]。建树 → 布局（文字按实际整形量宽）→ 按树序画。
//!
//! 节点的位置全由布局给出：流内节点按 flex / grid 排，高亮条、光标这类叠在别的节点上的用绝对定位。
//! 单位是像素（主题的点数在建树时已乘倍数），布局不取整，保证与直接算坐标的结果一致。

mod animated;
mod baseline;
mod box_paint;
mod cache_key;
mod draw;
mod effect;
mod extent;
mod fill;
mod icon;
mod keyed;
mod layer_place;
mod locate;
mod node;
mod paint;
mod split;
mod visual;

use std::cell::RefCell;
use std::collections::HashMap;

use taffy::prelude::TaffyMaxContent;
use taffy::{AlignItems, AvailableSpace, GridPlacement, Line, NodeId, Size, Style, TaffyTree};

use crate::error::RenderError;
use crate::text::{TextPainter, TextSize};
use cache_key::CacheKey;
use draw::EffectMask;
use tiny_skia::Pixmap;

pub(crate) use box_paint::BoxPaint;
pub(crate) use effect::{Effect, EffectKind};
pub(crate) use fill::Fill;
pub(crate) use icon::Icon;
pub(crate) use keyed::Keyed;
pub(crate) use layer_place::LayerPlace;
pub(crate) use node::SceneNode;
pub(crate) use split::SplitMark;
pub(crate) use visual::Visual;

pub(crate) struct Scene {
    /// 布局树，节点上下文是它画什么。
    tree: TaffyTree<SceneNode>,

    /// 算过的效果遮罩：动画帧重画时没动的节点直接复用（离屏图层里画的子树也用同一份，键里有画布大小）。
    effect_cache: RefCell<HashMap<CacheKey, Option<EffectMask>>>,

    /// 画好的图片填充框（九宫格窗口背景这类，每像素都要采样）：动画帧里没动就贴回去。
    visual_cache: RefCell<HashMap<CacheKey, CachedVisual>>,

    /// 分段模式：只画画序编号落在这个范围里的节点（见 `split.rs`）；`None` 为正常画。
    split: RefCell<Option<std::ops::Range<usize>>>,

    /// 分段模式下下一个节点的编号。
    order: std::cell::Cell<usize>,

    /// 分段模式下跳过的动画节点。
    marks: RefCell<Vec<SplitMark>>,

    /// 表格的各行（不含横跨整行的格子），布局时按基线对齐。
    table_rows: Vec<Vec<NodeId>>,
}

/// 缓存的一块画面：左上角在画布里的整数位置与位图。
struct CachedVisual {
    left: i32,

    top: i32,

    pixmap: Pixmap,
}

impl Scene {
    pub(crate) fn new() -> Self {
        let mut tree = TaffyTree::new();
        tree.disable_rounding();
        Self {
            tree,
            effect_cache: RefCell::new(HashMap::new()),
            visual_cache: RefCell::new(HashMap::new()),
            split: RefCell::new(None),
            order: std::cell::Cell::new(0),
            marks: RefCell::new(Vec::new()),
            table_rows: Vec::new(),
        }
    }

    /// 加一个节点，`children` 按画的先后排。
    pub(crate) fn node(
        &mut self,
        style: Style,
        visual: Visual,
        children: &[NodeId],
    ) -> Result<NodeId, RenderError> {
        let node = self.tree.new_with_children(style, children)?;
        self.tree.set_node_context(
            node,
            Some(SceneNode {
                visual,
                opacity: 1.0,
                effects: Vec::new(),
                transition: None,
                placed: None,
                animation: None,
                pose: None,
            }),
        )?;
        Ok(node)
    }

    /// 节点连同子节点的不透明度。
    pub(crate) fn set_opacity(&mut self, node: NodeId, opacity: f32) {
        if let Some(context) = self.tree.get_node_context_mut(node) {
            context.opacity = opacity.clamp(0.0, 1.0);
        }
    }

    /// 节点的投影、内阴影。
    pub(crate) fn set_effects(&mut self, node: NodeId, effects: Vec<Effect>) {
        if let Some(context) = self.tree.get_node_context_mut(node) {
            context.effects = effects;
        }
    }

    /// 把表格里的格子放到指定的行、列；`stretch` 时撑满所占的格子（横跨整行的高亮条）。
    pub(crate) fn place(
        &mut self,
        node: NodeId,
        row: Line<GridPlacement>,
        column: Line<GridPlacement>,
        stretch: bool,
    ) -> Result<(), RenderError> {
        let mut style = self.tree.style(node)?.clone();
        style.grid_row = row;
        style.grid_column = column;
        if stretch {
            style.justify_self = Some(AlignItems::STRETCH);
            style.align_self = Some(AlignItems::STRETCH);
        }
        self.tree.set_style(node, style)?;
        Ok(())
    }

    /// 以 `root` 为根按内容撑开算布局，返回根节点的宽高。表格行里的格子基线没对齐时补上边距再算一遍。
    pub(crate) fn layout(
        &mut self,
        root: NodeId,
        text: &mut TextPainter,
    ) -> Result<(f32, f32), RenderError> {
        // 布局会对同一个叶子按不同约束量好几次；单行文字的尺寸与约束无关，量一次记下来
        let mut measured: HashMap<NodeId, TextSize> = HashMap::new();
        self.compute(root, text, &mut measured)?;
        if self.align_baselines(&measured)? {
            self.compute(root, text, &mut measured)?;
        }
        let size = self.tree.layout(root)?.size;
        Ok((size.width, size.height))
    }

    fn compute(
        &mut self,
        root: NodeId,
        text: &mut TextPainter,
        measured: &mut HashMap<NodeId, TextSize>,
    ) -> Result<(), RenderError> {
        self.tree.compute_layout_with_measure(
            root,
            Size::MAX_CONTENT,
            |inputs, node, visual, style| {
                taffy::compute_leaf_layout(
                    inputs,
                    style,
                    |_, _| 0.0,
                    |known, _available: Size<AvailableSpace>| match visual {
                        Some(SceneNode {
                            visual:
                                Visual::Text {
                                    text: content,
                                    style,
                                },
                            ..
                        }) => {
                            let size = *measured
                                .entry(node)
                                .or_insert_with(|| text.measure(content, style));
                            Size {
                                width: known.width.unwrap_or(size.width),
                                height: known.height.unwrap_or(size.height),
                            }
                        }
                        _ => Size {
                            width: known.width.unwrap_or(0.0),
                            height: known.height.unwrap_or(0.0),
                        },
                    },
                )
            },
        )?;
        Ok(())
    }
}
