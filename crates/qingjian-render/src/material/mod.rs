//! 系统材质：主题写了 `material`、壳又垫得了时，窗口底下由壳铺一层系统实时模糊的材质，渲染器只画上面的内容。

mod backing;
mod kind;

pub use backing::MaterialBacking;
pub use kind::Material;
