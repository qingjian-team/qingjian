//! 矢量小图标：使用方实现这个 trait 自己画，引擎只负责把它放进盒子（边长 `size`，垂直居中、靠左）。

use std::fmt::Debug;

use crate::canvas::Canvas;
use crate::color::Color;

pub trait Icon: Debug + Send + Sync {
    /// 在左上角 `(x, y)`、边长 `size`（像素）的方块里画。
    fn draw(&self, canvas: &mut Canvas, x: f32, y: f32, size: f32, color: Color);
}
