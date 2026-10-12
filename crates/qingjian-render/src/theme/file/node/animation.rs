//! 节点的循环动画：`"animation": { "duration": 2400, "loop": true, "easing": "ease-in-out", "keyframes": [...] }`。
//! 关键帧写 `at`（0–1）与要动的属性（`x` / `y` 平移点数、`rotate` 度、`scale`、`opacity`），变换作用于整棵子树、不影响布局。

use serde::Deserialize;

use crate::animation::Easing;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct AnimationSpec {
    /// 一轮的时长（毫秒）。
    pub(crate) duration: f32,

    /// 循环播放；`false` 时播一轮停在最后一帧。
    #[serde(default = "default_loop", rename = "loop")]
    pub(crate) repeat: bool,

    /// 相邻两个关键帧之间的缓动。
    #[serde(default)]
    pub(crate) easing: Easing,

    pub(crate) keyframes: Vec<KeyframeSpec>,
}

fn default_loop() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub(crate) struct KeyframeSpec {
    /// 在一轮里的位置（0–1）。
    pub(crate) at: f32,

    pub(crate) x: Option<f32>,

    pub(crate) y: Option<f32>,

    /// 旋转（度，顺时针，绕节点中心）。
    pub(crate) rotate: Option<f32>,

    /// 缩放（绕节点中心）。
    pub(crate) scale: Option<f32>,

    pub(crate) opacity: Option<f32>,
}
