//! 输入法图标：系统语言栏 / 输入指示器上「青简」旁边那个。
//!
//! TSF 的 `AddLanguageProfile` 要一个图标文件路径（`.ico`，或带资源的 DLL / EXE 加索引）。图标不嵌进 DLL
//! 资源（那要 rc.exe / windres 参与构建，本机交叉 check 就没法做），而是随 DLL 编译进来，注册时写成
//! 一个机器级的 `.ico` 文件（regsvr32 本来就要管理员）。

use std::path::PathBuf;

/// 与 macOS 端同一张 logo（`assets/icon/logo.png`）生成的多尺寸 `.ico`。
const ICON: &[u8] = include_bytes!("../../resources/qingjian.ico");

/// 图标落地的位置：`%ProgramData%\Qingjian\qingjian.ico`。用户级目录不行——注册是机器级的，别的用户也要能读到。
fn path() -> Option<PathBuf> {
    std::env::var_os("ProgramData")
        .map(|base| PathBuf::from(base).join("Qingjian").join("qingjian.ico"))
}

/// 把图标写到机器级目录，返回路径。写不了返回 `None`，调用方就注册成无图标。
pub(super) fn install() -> Option<PathBuf> {
    let path = path()?;
    let dir = path.parent()?;
    if let Err(error) = std::fs::create_dir_all(dir).and_then(|()| std::fs::write(&path, ICON)) {
        super::log::log(&format!("写图标文件失败，注册成无图标: {error}"));
        return None;
    }
    Some(path)
}

/// 反注册时删掉图标文件，尽力而为。
pub(super) fn uninstall() {
    if let Some(path) = path() {
        let _ = std::fs::remove_file(path);
    }
}
