//! 一张预览图的结果。

use std::path::PathBuf;

use super::ShotError;

#[derive(Debug)]
pub struct Shot {
    /// 文件名：`vertical.png` / `horizontal.png` / `vertical-dark.png`。
    pub file: &'static str,

    /// 写出的路径，或这张为什么没画出来。
    pub result: Result<PathBuf, ShotError>,
}
