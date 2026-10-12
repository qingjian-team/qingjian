//! 节点的过渡：`"transition": { "duration": 120, "easing": "ease-out" }`，按 `id` 与上一帧配对，位置、大小、不透明度插值。

use serde::Deserialize;

use crate::animation::Easing;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub(crate) struct TransitionSpec {
    /// 时长（毫秒）。
    pub(crate) duration: f32,

    #[serde(default)]
    pub(crate) easing: Easing,
}
