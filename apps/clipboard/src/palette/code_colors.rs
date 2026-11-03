//! [`CodeColors`]：代码高亮的颜色。

use bamboo_slip::Color;

/// 代码高亮的颜色。
pub struct CodeColors {
    pub keyword: Color,

    pub string: Color,

    pub comment: Color,

    pub number: Color,
}
