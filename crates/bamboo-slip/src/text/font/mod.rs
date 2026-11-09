//! 字体用法：一种文字用什么字号、行高、字重、斜体与字族回退链。

mod family;
mod spec;
mod weight;

pub use family::{FamilyId, FamilyList, FamilyTable};
pub use spec::FontSpec;
pub use weight::FontWeight;
