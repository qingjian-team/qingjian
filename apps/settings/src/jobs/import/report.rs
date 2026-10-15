//! 一次导入的结果。

use crate::library::Kind;

pub struct Report {
    pub kind: Kind,

    /// 导入成功的文件名（不含扩展名）：之前关掉过的要重新打开。
    pub imported: Vec<String>,

    /// 给人看的结果，一个文件一行。
    pub lines: Vec<String>,
}
