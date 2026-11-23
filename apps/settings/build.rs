//! 在 Windows 上编时把青简图标嵌进 exe（开始菜单、任务栏、Alt+Tab 里显示的就是它）。

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

/// 图标资源要 `rc.exe`（MSVC）编，只在 Windows 宿主上做；失败只警告，别让编译挂掉。
#[cfg(windows)]
fn embed_icon() {
    const ICON: &str = "../windows/tsf/resources/qingjian.ico";
    println!("cargo:rerun-if-changed={ICON}");
    if let Err(error) = winresource::WindowsResource::new().set_icon(ICON).compile() {
        println!("cargo:warning=嵌入设置程序图标失败: {error}");
    }
}

#[cfg(not(windows))]
fn embed_icon() {}
