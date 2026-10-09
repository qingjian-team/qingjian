//! 关键帧动画（像素已换算）：按时间求姿态。每个属性只在写了它的关键帧之间插值，没写过的取缺省值（不动）。

use std::time::Duration;

use super::{Easing, Pose};
use crate::theme::file::node::{AnimationSpec, KeyframeSpec};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Keyframes {
    duration: Duration,

    repeat: bool,

    easing: Easing,

    /// 按 `at` 排好，平移已乘倍数。
    frames: Vec<KeyframeSpec>,
}

impl Keyframes {
    pub(crate) fn from_spec(spec: &AnimationSpec, scale: f32) -> Self {
        let mut frames: Vec<KeyframeSpec> = spec
            .keyframes
            .iter()
            .map(|frame| KeyframeSpec {
                at: frame.at.clamp(0.0, 1.0),
                x: frame.x.map(|x| x * scale),
                y: frame.y.map(|y| y * scale),
                ..*frame
            })
            .collect();
        frames.sort_by(|a, b| a.at.total_cmp(&b.at));
        Self {
            duration: Duration::from_secs_f32(spec.duration.max(1.0) / 1000.0),
            repeat: spec.repeat,
            easing: spec.easing,
            frames,
        }
    }

    /// 动画开始后 `elapsed` 时刻的姿态。
    pub(crate) fn pose(&self, elapsed: Duration) -> Pose {
        let turns = elapsed.as_secs_f32() / self.duration.as_secs_f32();
        let t = if self.repeat {
            turns.fract()
        } else {
            turns.min(1.0)
        };
        Pose {
            x: self.value(t, |frame| frame.x, 0.0),
            y: self.value(t, |frame| frame.y, 0.0),
            rotate: self.value(t, |frame| frame.rotate, 0.0),
            scale: self.value(t, |frame| frame.scale, 1.0),
            opacity: self.value(t, |frame| frame.opacity, 1.0).clamp(0.0, 1.0),
        }
    }

    /// 播完了（不循环且过了一轮），之后不用再要帧。
    pub(crate) fn finished(&self, elapsed: Duration) -> bool {
        !self.repeat && elapsed >= self.duration
    }

    /// 一轮里任意时刻平移的最大绝对值、缩放的最大值：算画出范围用。
    pub(crate) fn reach(&self) -> (f32, f32, f32) {
        let mut reach = (0.0_f32, 0.0_f32, 1.0_f32);
        for frame in &self.frames {
            reach.0 = reach.0.max(frame.x.unwrap_or(0.0).abs());
            reach.1 = reach.1.max(frame.y.unwrap_or(0.0).abs());
            reach.2 = reach.2.max(frame.scale.unwrap_or(1.0));
        }
        reach
    }

    /// 有旋转。
    pub(crate) fn rotates(&self) -> bool {
        self.frames
            .iter()
            .any(|frame| frame.rotate.is_some_and(|r| r != 0.0))
    }

    /// 属性在 `t` 处的值：找前后两个写了它的关键帧插值。开头、结尾没写它时按缺省值补（同 CSS 缺 0% / 100% 关键帧）。
    fn value(&self, t: f32, get: impl Fn(&KeyframeSpec) -> Option<f32>, rest: f32) -> f32 {
        let mut defined: Vec<(f32, f32)> = self
            .frames
            .iter()
            .filter_map(|frame| get(frame).map(|value| (frame.at, value)))
            .collect();
        if defined.is_empty() {
            return rest;
        }
        if defined.first().is_some_and(|first| first.0 > 0.0) {
            defined.insert(0, (0.0, rest));
        }
        if defined.last().is_some_and(|last| last.0 < 1.0) {
            defined.push((1.0, rest));
        }
        let (first, last) = (defined[0], defined[defined.len() - 1]);
        if t <= first.0 {
            return first.1;
        }
        if t >= last.0 {
            return last.1;
        }
        let Some(pair) = defined
            .windows(2)
            .find(|pair| t >= pair[0].0 && t <= pair[1].0)
        else {
            return last.1;
        };
        let ((a_at, a), (b_at, b)) = (pair[0], pair[1]);
        let span = b_at - a_at;
        let progress = if span > 0.0 {
            self.easing.apply((t - a_at) / span)
        } else {
            1.0
        };
        a + (b - a) * progress
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(json: &str) -> AnimationSpec {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn interpolates_each_property_between_its_own_keyframes() {
        let frames = Keyframes::from_spec(
            &spec(
                r#"{ "duration": 1000, "easing": "linear", "keyframes": [
                    { "at": 0, "y": 0 }, { "at": 0.5, "y": -4, "opacity": 0.5 }, { "at": 1, "y": 0 } ] }"#,
            ),
            2.0,
        );
        let pose = frames.pose(Duration::from_millis(250));
        assert!(
            (pose.y - -4.0).abs() < 1e-4,
            "平移按倍数换成像素：-4pt×2 的一半"
        );
        assert!(
            (pose.opacity - 0.75).abs() < 1e-4,
            "只在中间写的属性两头按缺省值补"
        );
        assert_eq!(pose.x, 0.0);
        // 循环：1.25 轮与 0.25 轮一样
        assert_eq!(frames.pose(Duration::from_millis(1250)), pose);
    }

    #[test]
    fn once_holds_last_frame() {
        let frames = Keyframes::from_spec(
            &spec(
                r#"{ "duration": 100, "loop": false, "keyframes": [{ "at": 0, "x": 0 }, { "at": 1, "x": 5 }] }"#,
            ),
            1.0,
        );
        assert_eq!(frames.pose(Duration::from_millis(500)).x, 5.0);
        assert!(frames.finished(Duration::from_millis(500)));
    }
}
