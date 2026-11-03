//! [`WorkArea`]：面板能占的屏幕区域。

/// 显示器去掉任务栏后的区域（物理像素）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkArea {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}
