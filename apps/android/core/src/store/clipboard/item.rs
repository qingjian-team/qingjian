//! 交给壳的一条剪贴板历史。

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ClipboardItem {
    /// 文字原样（图片条目为空）。
    pub text: String,

    /// 图片文件的完整路径；文字条目为 `None`。
    pub image: Option<String>,
}
