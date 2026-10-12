//! 渲染器留住的上一帧：场景树、画布几何、各配对节点的位置与在播的过渡。下一帧拿它找过渡起点，动画帧拿它重画。

use std::collections::HashMap;
use std::time::Instant;

use taffy::NodeId;

use crate::animation::{Keyframes, Placement, Transition};
use crate::layout::Layout;
use crate::scene::Scene;

use super::HitRegion;
use super::partial::Partial;

pub(super) struct Retained {
    pub(super) scene: Scene,

    pub(super) root: NodeId,

    /// 画这一帧用的排布与倍数；变了就不跟新帧配对（位置不可比）。
    pub(super) layout: Layout,

    pub(super) scale: f32,

    /// 位图宽高（像素），动画期间不变。
    pub(super) size: (u32, u32),

    /// 根节点左上角在位图里的位置（像素，整数）。
    pub(super) origin: (f32, f32),

    /// 内容区宽高（像素，已取整）。
    pub(super) content: (f32, f32),

    /// 带过渡的节点布局给的位置（相对根节点），按配对名。
    pub(super) placements: HashMap<String, Placement>,

    /// 在播的过渡。
    pub(super) transitions: Vec<Transition>,

    /// 带循环动画的节点与关键帧。
    pub(super) animated: Vec<(NodeId, Keyframes)>,

    /// 循环动画局部重画的缓存；整张重画后作废。
    pub(super) partial: Option<Partial>,

    /// 候选与译词的点击区域，动画帧照抄（布局不变）。
    pub(super) hits: Vec<HitRegion>,
}

impl Retained {
    /// `now` 时刻屏幕上各配对节点的实际位置：在播的取插值，其余取布局位置。新一帧的过渡从这里出发，
    /// 所以过渡没播完又来一帧时从半路接着走，不会跳回起点。
    pub(super) fn shown(&self, now: Instant) -> HashMap<String, Placement> {
        let mut shown = self.placements.clone();
        for transition in &self.transitions {
            shown.insert(transition.key.clone(), transition.at(now));
        }
        shown
    }
}
