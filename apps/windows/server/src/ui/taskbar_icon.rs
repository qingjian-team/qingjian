//! 输入法登记的图标文件（`%ProgramData%\Qingjian\qingjian.ico`）跟着任务栏深浅换。DLL 注册时按当时的深浅写一版，
//! 之后由这里在 Server 启动与系统设置变更（`WM_SETTINGCHANGE`，深浅色切换会广播）时改写；任务栏在下次切换输入法时重读这个文件，那时才换过来（不重新注册就没法让它立刻刷新，注册要管理员）。
//! 目录由安装包放开给普通用户写，写不了只记日志。图标是机器级的，几个用户同时登录、深浅不同时以最后改写的为准。

use std::path::PathBuf;

/// 白墨，深色任务栏用；与 DLL 注册时写的是同一份。
const ICON_DARK_TASKBAR: &[u8] = include_bytes!("../../../tsf/resources/qingjian-mono-dark.ico");

/// 深绿墨，浅色任务栏用。
const ICON_LIGHT_TASKBAR: &[u8] = include_bytes!("../../../tsf/resources/qingjian-mono-light.ico");

/// 文件已是当前深浅那版就不动；没注册过（文件不在）也不建，免得卸载后留下孤儿文件。
pub(super) fn sync() {
    let Some(path) = path() else {
        return;
    };
    let wanted = if taskbar_is_light() {
        ICON_LIGHT_TASKBAR
    } else {
        ICON_DARK_TASKBAR
    };
    match std::fs::read(&path) {
        Ok(current) if current == wanted => {}
        Ok(_) => {
            if let Err(error) = std::fs::write(&path, wanted) {
                tracing::warn!(%error, path = %path.display(), "failed to rewrite taskbar icon");
            }
        }
        Err(_) => {}
    }
}

fn path() -> Option<PathBuf> {
    std::env::var_os("ProgramData")
        .map(|base| PathBuf::from(base).join("Qingjian").join("qingjian.ico"))
}

/// 与 DLL 画「中 / 英」图标同一个判断：`SystemUsesLightTheme = 1` 是浅色任务栏，读不到按深色。
fn taskbar_is_light() -> bool {
    windows_registry::CURRENT_USER
        .open(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|key| key.get_u32("SystemUsesLightTheme"))
        .is_ok_and(|value| value == 1)
}
