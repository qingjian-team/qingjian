//! 在 Windows 上编时把图标嵌进 exe：资源 id 1 是程序图标，2、3 是深色、浅色任务栏的托盘图标（见 `system/tray`）。

fn main() {
    refuse_system_drive();
    embed_icon();
}

/// 构建输出落在 C 盘就停（见 docs/contributing.md 的「构建环境」）。
#[cfg(windows)]
fn refuse_system_drive() {
    if std::env::var_os("QINGJIAN_ALLOW_C_TARGET").is_some() {
        return;
    }
    let Some(out) = std::env::var_os("OUT_DIR") else {
        return;
    };
    let path = std::path::Path::new(&out);
    let Some(std::path::Component::Prefix(prefix)) = path.components().next() else {
        return;
    };
    if prefix
        .as_os_str()
        .to_string_lossy()
        .eq_ignore_ascii_case("C:")
    {
        panic!(
            "禁止在 C 盘编译：把 CARGO_TARGET_DIR 指到非系统盘（如 D:\\qingjian-target），\
             或设 QINGJIAN_ALLOW_C_TARGET=1 放行（见 docs/contributing.md）"
        );
    }
}

#[cfg(not(windows))]
fn refuse_system_drive() {}

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
