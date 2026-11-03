//! 在 Windows 上编时把青简图标嵌进 exe，托盘图标按资源 id 1 取。

fn main() {
    embed_icon();
}

/// 图标资源要 `rc.exe`（MSVC）编，只在 Windows 宿主上做；失败只警告，托盘退回系统图标。
#[cfg(windows)]
fn embed_icon() {
    const ICON: &str = "../windows/tsf/resources/qingjian.ico";
    println!("cargo:rerun-if-changed={ICON}");
    if let Err(error) = winresource::WindowsResource::new()
        .set_icon_with_id(ICON, "1")
        .compile()
    {
        println!("cargo:warning=failed to embed the clipboard icon: {error}");
    }
}

#[cfg(not(windows))]
fn embed_icon() {}
