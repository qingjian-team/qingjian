//! [`Editing`]：正在编辑的一条。

use bamboo_slip::TextEdit;

/// 正在编辑的一条。
pub struct Editing {
    pub id: String,

    pub text: TextEdit,
}
