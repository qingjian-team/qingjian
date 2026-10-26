//! 布局之后的一个可交互元素：在根坐标里的盒子（带上级裁剪）与最近的可交互上级。

use taffy::NodeId;

use super::{Interact, Key};
use crate::scene::Located;

pub struct Element<M> {
    pub key: Key,

    pub interact: Interact<M>,

    pub located: Located,

    /// 最近的可交互上级在元素表里的下标。
    pub parent: Option<usize>,
}

impl<M> Element<M> {
    pub fn node(&self) -> NodeId {
        self.located.node
    }

    /// 左、上、宽、高（像素）。
    pub fn rect(&self) -> (f32, f32, f32, f32) {
        self.located.rect
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        self.located.contains(x, y)
    }
}
