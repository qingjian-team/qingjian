//! 动画：缓动曲线、过渡的起止与插值。[`crate::Stage`] 留住上一帧，按节点的配对名算过渡，使用方按 [`crate::Painted::next_frame`] 定时要下一帧。

mod easing;
mod keyframe;
mod keyframes;
mod placement;
mod pose;
mod transition;

pub use easing::Easing;
pub use keyframe::Keyframe;
pub use keyframes::Keyframes;
pub use placement::Placement;
pub use pose::Pose;
pub use transition::Transition;

use std::time::Duration;

/// 过渡在播时两帧的间隔（约 60 帧）。
pub const FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// 只有循环动画在播时两帧的间隔（上限 30 帧，省电）。
pub const LOOP_INTERVAL: Duration = Duration::from_millis(33);
