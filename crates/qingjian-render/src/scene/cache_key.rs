//! 画面缓存的键：哪个节点的哪一份（第几个效果的遮罩，或节点自己的画面）、盒子在画布里的位置大小、画布大小。
//! 都没变就直接复用：动画帧里不动的节点不再重画形状、重做模糊、重新采样图片。

use taffy::NodeId;

use crate::canvas::Canvas;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct CacheKey {
    node: NodeId,

    slot: Slot,

    /// 盒子 `(x, y, 宽, 高)` 的位模式（像素，浮点原样比）。
    rect: [u32; 4],

    canvas: (u32, u32),
}

/// 缓存的是节点的哪一份。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Slot {
    /// 第几个效果的遮罩。
    Effect(usize),

    /// 节点自己的画面（图片填充的框）。
    Visual,

    /// 带循环动画的节点整棵子树（未变换），每帧按姿态变换后贴上。
    Layer,
}

impl CacheKey {
    pub(super) fn new(
        node: NodeId,
        slot: Slot,
        rect: (f32, f32, f32, f32),
        canvas: &Canvas,
    ) -> Self {
        Self::with_size(node, slot, rect, (canvas.width(), canvas.height()))
    }

    /// 画布大小直接给（往局部小图上画、但要与整张画布共用缓存时）。
    pub(super) fn with_size(
        node: NodeId,
        slot: Slot,
        rect: (f32, f32, f32, f32),
        canvas: (u32, u32),
    ) -> Self {
        Self {
            node,
            slot,
            rect: [
                rect.0.to_bits(),
                rect.1.to_bits(),
                rect.2.to_bits(),
                rect.3.to_bits(),
            ],
            canvas,
        }
    }

    pub(super) fn node(&self) -> NodeId {
        self.node
    }
}
