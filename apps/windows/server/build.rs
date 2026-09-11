//! 嵌 uiAccess manifest：候选窗口要盖过商店 / 任务栏搜索这些高 z-band 宿主，`SetWindowPos(HWND_TOPMOST)`
//! 才能升进 UIAccess 高带。系统只对签名且装在 Program Files 的 exe 授予，光有 manifest 不够。
//! manifest 缺省含 PerMonitorV2 DPI 感知，与运行时那次 `SetProcessDpiAwarenessContext` 一致。
//! 另把青简图标嵌进 exe（任务管理器 / 启动项里显示）。

use embed_manifest::manifest::ExecutionLevel;
use embed_manifest::{embed_manifest, new_manifest};

fn main() {
    // build.rs 跑在宿主机上，只有目标是 Windows 时才嵌。
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let manifest = new_manifest("Qingjian.Server")
            .requested_execution_level(ExecutionLevel::AsInvoker)
            .ui_access(true);
        embed_manifest(manifest).expect("嵌入 Server uiAccess manifest 失败");
    }
    embed_icon();
    println!("cargo:rerun-if-changed=build.rs");
}

/// 图标资源要 `rc.exe`（MSVC）编，只在 Windows 宿主上做；失败只警告，别让编译挂掉。
/// winresource 缺省不带 manifest，与上面链接器嵌的那份不冲突。
#[cfg(windows)]
fn embed_icon() {
    const ICON: &str = "../tsf/resources/qingjian.ico";
    println!("cargo:rerun-if-changed={ICON}");
    if let Err(error) = winresource::WindowsResource::new().set_icon(ICON).compile() {
        println!("cargo:warning=嵌入 Server 图标失败: {error}");
    }
}

#[cfg(not(windows))]
fn embed_icon() {}
