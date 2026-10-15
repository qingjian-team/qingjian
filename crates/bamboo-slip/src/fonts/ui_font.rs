//! 用户指定的界面字体：字族名加它的字体文件。文件由使用方按平台的字体登记查出来（macOS 走 CoreText，Windows 见 `directwrite`），引擎只负责加载。

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiFont {
    /// 字族名，如 `LXGW WenKai`。
    pub family: String,

    /// 这个字族的字体文件；空表示没找到。
    pub files: Vec<PathBuf>,
}
