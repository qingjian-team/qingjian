//! 一个关键帧：在一轮里的位置与这一刻要到的属性值；没写的属性不参与这一帧的插值。

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Keyframe {
    /// 在一轮里的位置（0–1）。
    pub at: f32,

    /// 平移（像素）。
    pub x: Option<f32>,

    pub y: Option<f32>,

    /// 旋转（度，顺时针，绕节点中心）。
    pub rotate: Option<f32>,

    /// 缩放（绕节点中心）。
    pub scale: Option<f32>,

    pub opacity: Option<f32>,
}
