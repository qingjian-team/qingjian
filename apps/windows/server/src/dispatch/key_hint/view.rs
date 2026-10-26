//! 随一帧候选交给 UI 线程的键位提示。

use super::KeyHintContent;

#[derive(Debug, Clone, PartialEq)]
pub struct KeyHintView {
    pub content: KeyHintContent,

    /// 窗口不透明度（0.3–1）。
    pub opacity: f32,

    /// 键盘大小（1 为原大）。
    pub zoom: f32,
}
