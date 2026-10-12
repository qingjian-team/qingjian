//! 动画：缓动曲线、过渡的起止与插值。渲染器留住上一帧，按节点 `id` 配对算过渡，壳按 [`crate::Rendered::next_frame`] 定时要下一帧。

mod easing;
mod keyframes;
mod placement;
mod pose;
mod transition;

pub(crate) use easing::Easing;
pub(crate) use keyframes::Keyframes;
pub(crate) use placement::Placement;
pub(crate) use pose::Pose;
pub(crate) use transition::Transition;

use std::time::Duration;

/// 过渡在播时两帧的间隔（约 60 帧）。
pub(crate) const FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// 只有循环动画在播时两帧的间隔（上限 30 帧，省电）。
pub(crate) const LOOP_INTERVAL: Duration = Duration::from_millis(33);
