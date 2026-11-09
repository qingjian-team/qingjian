//! 输入法图标：`AddLanguageProfile` 要一个图标文件路径。不嵌 DLL 资源（要 rc.exe / windres 参与构建，
//! 本机交叉 check 做不了），而是编译进来、注册时写成机器级 `.ico`（regsvr32 本来就要管理员）。
//!
//! 任务栏输入指示与 Win + Space 列表显示的就是它，所以用无底板的单色版，按注册时任务栏的深浅挑一版，之后由 Server 跟着深浅色改写；
//! 开始菜单、安装包等带底板的场合仍用彩色的 `qingjian.ico`（嵌在各 exe 与安装包里，与这里无关）。

use std::path::PathBuf;

use crate::com::mode::taskbar_is_light;

/// 白墨，深色任务栏用。
const ICON_DARK_TASKBAR: &[u8] = include_bytes!("../../../resources/qingjian-mono-dark.ico");

/// 深绿墨，浅色任务栏用。
const ICON_LIGHT_TASKBAR: &[u8] = include_bytes!("../../../resources/qingjian-mono-light.ico");

/// `%ProgramData%\Qingjian\qingjian.ico`。注册是机器级的，别的用户也要能读到，所以不能放用户目录。
fn path() -> Option<PathBuf> {
    std::env::var_os("ProgramData")
        .map(|base| PathBuf::from(base).join("Qingjian").join("qingjian.ico"))
}

/// 写不了返回 `None`，调用方注册成无图标。
pub(super) fn install() -> Option<PathBuf> {
    let path = path()?;
    let dir = path.parent()?;
    let icon = if taskbar_is_light() {
        ICON_LIGHT_TASKBAR
    } else {
        ICON_DARK_TASKBAR
    };
    // 先删再写：新建的文件才继承安装包给目录放开的写权限，Server 跟着深浅色改写要用（老版本留下的文件没有）
    let _ = std::fs::remove_file(&path);
    if let Err(error) = std::fs::create_dir_all(dir).and_then(|()| std::fs::write(&path, icon)) {
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
