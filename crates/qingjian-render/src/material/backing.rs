//! 这一帧要壳垫的材质：种类、圆角，以及文字的深浅听谁的。

use super::Material;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialBacking {
    pub kind: Material,

    /// 内容区的圆角（像素）。
    pub radius: f32,

    /// 文字跟着材质反色：玻璃随背后内容变深变浅，壳按玻璃的深浅取主题重画。
    /// 为假时壳要把材质钉在 [`Self::locked_dark`] 或系统外观上，别让它自己变色。
    pub follows_backdrop: bool,

    /// 主题锁定的外观：`Some(true)` 只有深色、`Some(false)` 只有浅色，`None` 跟系统。
    pub locked_dark: Option<bool>,
}
