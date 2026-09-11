//! 输入法图标：`AddLanguageProfile` 要一个图标文件路径。不嵌 DLL 资源（要 rc.exe / windres 参与构建，
//! 本机交叉 check 做不了），而是编译进来、注册时写成机器级 `.ico`（regsvr32 本来就要管理员）。

use std::path::PathBuf;

/// 与 macOS 端同一张 logo 生成的多尺寸 `.ico`。
const ICON: &[u8] = include_bytes!("../../../resources/qingjian.ico");

/// `%ProgramData%\Qingjian\qingjian.ico`。注册是机器级的，别的用户也要能读到，所以不能放用户目录。
fn path() -> Option<PathBuf> {
    std::env::var_os("ProgramData")
        .map(|base| PathBuf::from(base).join("Qingjian").join("qingjian.ico"))
}

/// 写不了返回 `None`，调用方注册成无图标。
pub(super) fn install() -> Option<PathBuf> {
    let path = path()?;
    let dir = path.parent()?;
    if let Err(error) = std::fs::create_dir_all(dir).and_then(|()| std::fs::write(&path, ICON)) {
        crate::com::log::log(&format!("写图标文件失败，注册成无图标: {error}"));
        return None;
    }
    Some(path)
}

pub(super) fn uninstall() {
    if let Some(path) = path() {
        let _ = std::fs::remove_file(path);
    }
}
