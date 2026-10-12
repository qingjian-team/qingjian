//! 主题里颜色的几种写法：颜色引用（`#rrggbb[aa]` 或 `@名字`）、节点上按条件二选一、`variables` 里的颜色值。

mod reference;
mod spec;
mod value;

pub(crate) use reference::{ColorRef, parse_hex};
pub(crate) use spec::ColorSpec;
pub(crate) use value::ColorValue;
