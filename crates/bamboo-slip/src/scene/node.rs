//! 场景树节点的上下文：画什么、效果、整棵子树的不透明度，以及过渡用的配对名与动画中的位置。

use super::effect::Effect;
use super::visual::Visual;
use crate::animation::{Easing, Keyframes, Placement, Pose};

#[derive(Debug, Clone)]
pub struct SceneNode {
    pub visual: Visual,

    /// 不透明度（0–1），小于 1 时子树先画到离屏图层再合成。
    pub opacity: f32,

    /// 投影、内阴影，按写的顺序画。
    pub effects: Vec<Effect>,

    /// 过渡配对名（同名节点跨帧配对，重复时带序号）与过渡参数；没写 `transition` 的为 `None`。
    pub transition: Option<(String, std::time::Duration, Easing)>,

    /// 过渡只渐变颜色与不透明度，位置、大小直接到布局给的（悬停这类状态变化，跟着排版挪动时不该拖后）。
    pub fade: bool,

    /// 子孙的滑动过渡以它为参照：它自己挪了不算子孙在动，只有相对它的位置变了才滑。
    pub anchor: bool,

    /// 动画中的位置（画布像素）与不透明度，盖过布局算的；不在动画中为 `None`。
    pub placed: Option<Placement>,

    /// 循环动画的关键帧。
    pub animation: Option<Keyframes>,

    /// 这一帧的姿态（循环动画算出来的），整棵子树跟着变换；`None` 为原样。
    pub pose: Option<Pose>,

    /// 子节点只画在这个节点的盒子里（圆角半径，像素）；`None` 不裁。节点自己的画面与效果不受影响。
    pub clip: Option<f32>,
}
