//! 在 Windows 上编时把青简图标嵌进 exe（开始菜单、任务栏、Alt+Tab 里显示的就是它）。

fn main() {
    embed_icon();
}

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
