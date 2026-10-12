//! 宽度 + 颜色：框的边框（画在盒子内侧），也是文字的描边（向外，垫在字形底下）；都不占布局空间。

use serde::Deserialize;

use crate::theme::file::ColorSpec;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct BorderSpec {
    /// 宽度（点）。
    pub(crate) width: f32,

    pub(crate) color: ColorSpec,
}
