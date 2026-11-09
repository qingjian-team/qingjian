//! 在 Windows 上编时把图标嵌进 exe：资源 id 1 是程序图标，2、3 是深色、浅色任务栏的托盘图标（见 `system/tray`）。

fn main() {
    embed_icon();
}

/// 图标资源要 `rc.exe`（MSVC）编，只在 Windows 宿主上做；失败只警告，托盘退回系统图标。
#[cfg(windows)]
fn embed_icon() {
    const ICON: &str = "../windows/tsf/resources/qingjian.ico";
    const TRAY_DARK: &str = "resources/tray-dark.ico";
    const TRAY_LIGHT: &str = "resources/tray-light.ico";
    for icon in [ICON, TRAY_DARK, TRAY_LIGHT] {
        println!("cargo:rerun-if-changed={icon}");
    }
    if let Err(error) = winresource::WindowsResource::new()
        .set_icon_with_id(ICON, "1")
        .set_icon_with_id(TRAY_DARK, "2")
        .set_icon_with_id(TRAY_LIGHT, "3")
        .compile()
    {
        println!("cargo:warning=failed to embed the clipboard icon: {error}");
    }
}

#[cfg(not(windows))]
fn embed_icon() {}
