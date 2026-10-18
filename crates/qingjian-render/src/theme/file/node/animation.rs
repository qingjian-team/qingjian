//! 节点的循环动画：`"animation": { "duration": 2400, "loop": true, "easing": "ease-in-out", "keyframes": [...] }`。
//! 关键帧写 `at`（0–1）与要动的属性（`x` / `y` 平移点数、`rotate` 度、`scale`、`opacity`），变换作用于整棵子树、不影响布局。

use std::time::Duration;

use bamboo_slip::{Easing, Keyframe, Keyframes};
use serde::Deserialize;

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

impl AnimationSpec {
    /// 平移按倍数换成像素，时长至少 1 毫秒。
    pub(crate) fn keyframes(&self, scale: f32) -> Keyframes {
        let frames = self
            .keyframes
            .iter()
            .map(|frame| Keyframe {
                at: frame.at,
                x: frame.x.map(|x| x * scale),
                y: frame.y.map(|y| y * scale),
                rotate: frame.rotate,
                scale: frame.scale,
                opacity: frame.opacity,
            })
            .collect();
        Keyframes::new(
            Duration::from_secs_f32(self.duration.max(1.0) / 1000.0),
            self.repeat,
            self.easing,
            frames,
        )
    }
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::AnimationSpec;

    #[test]
    fn translation_is_scaled_to_pixels() {
        let spec: AnimationSpec = serde_json::from_str(
            r#"{ "duration": 1000, "easing": "linear", "keyframes": [
                { "at": 0, "y": 0 }, { "at": 0.5, "y": -4 }, { "at": 1, "y": 0 } ] }"#,
        )
        .unwrap();
        let pose = spec.keyframes(2.0).pose(Duration::from_millis(250));
        assert!((pose.y - -4.0).abs() < 1e-4, "-4pt×2 的一半");
    }
}
