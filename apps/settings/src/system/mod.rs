//! 与操作系统打交道的部分：文件位置、本机字体、打开文件与目录的按钮动作、Windows 上只开一个窗口与系统的 Ctrl + Space。

pub mod action;
pub mod fonts;
#[cfg(windows)]
pub mod hotkey;
pub mod paths;
#[cfg(windows)]
pub mod single_instance;
