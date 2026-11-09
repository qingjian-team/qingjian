//! 五笔字根图：随包的 86 版字根字形（`assets/schemes/wubi86/roots86-glyphs.tsv`，思源黑体的轮廓，SIL OFL）按键排好，给壳画在候选窗旁边。
//! 只给数据（每个字根一段 SVG 路径与露出区域），怎么画由渲染器决定。

mod chart;
mod glyph;
mod key;

pub use chart::RootChart;
pub use glyph::RootGlyph;
pub use key::RootKey;
