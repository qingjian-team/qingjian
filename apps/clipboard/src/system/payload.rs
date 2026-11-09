//! [`Payload`]：要写回系统剪贴板的一条内容。

use qingjian_clipboard::{ClipFormat, ClipKind};

/// 要写回系统剪贴板的一条内容。
pub struct Payload {
    pub kind: ClipKind,

    pub text: String,

    pub formats: Vec<ClipFormat>,
}
