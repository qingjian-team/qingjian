//! 一个字根的字形。

#[derive(Debug, Clone, PartialEq)]
pub struct RootGlyph {
    /// 取自哪个字（部件从常用字里取），读字根表时对照用。
    pub source: &'static str,

    /// SVG 路径，字面方块 0–1000、y 向下，只有绝对坐标的 `M L Q C Z`。
    pub svg: &'static str,

    /// 部件露出的区域（左、上、右、下）；`None` 是整字。
    pub frame: Option<[f32; 4]>,
}
