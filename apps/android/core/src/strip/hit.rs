//! 候选条位图上的一块可点区域。

use qingjian_render::{HitRegion, HitTarget};
use serde::Serialize;

/// 位图像素坐标；`sense` 有值时是这个候选下面的第几条译文。
#[derive(Debug, Serialize)]
pub struct StripHit {
    pub candidate: usize,

    pub sense: Option<usize>,

    pub x: f32,

    pub y: f32,

    pub width: f32,

    pub height: f32,
}

impl StripHit {
    /// 区域是内容区坐标，加上内容区在位图里的偏移。候选条没有右键菜单，菜单项给 `None`。
    pub fn new(region: &HitRegion, offset: (u32, u32)) -> Option<Self> {
        let (candidate, sense) = match region.target {
            HitTarget::Candidate(candidate) => (candidate, None),
            HitTarget::Translation { candidate, sense } => (candidate, Some(sense)),
            HitTarget::MenuItem(_) => return None,
        };
        Some(Self {
            candidate,
            sense,
            x: region.x + offset.0 as f32,
            y: region.y + offset.1 as f32,
            width: region.width,
            height: region.height,
        })
    }
}
