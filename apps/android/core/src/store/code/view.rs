//! 交给壳的代码显示信息。

use qingjian_core::code::CodeKind;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CodeView {
    /// 像代码：等宽、不重排。
    pub code: bool,

    /// 高亮片段：UTF-16 的起止与种类，按出现顺序。
    pub spans: Vec<(usize, usize, CodeKind)>,
}
