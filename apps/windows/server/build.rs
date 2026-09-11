//! 构建期把 uiAccess manifest 嵌进 Server exe。
//!
//! 候选窗口在本进程自绘，普通置顶窗被微软商店 / 任务栏搜索这些**更高 z-band** 的宿主盖住。
//! `uiAccess="true"` 让本进程的 `SetWindowPos(HWND_TOPMOST)` 自动升进 UIAccess 高带盖过它们——
//! 但 Windows 只对**代码签名**且装在**安全位置**（Program Files / Windows）的 exe 授予该权限，
//! 光有 manifest 不够（开发期自签 + box 装进受信任的根，发版换 Certum 证书；见记忆 candidate-ui-refactor）。
//!
//! `new_manifest` 的缺省已含 PerMonitorV2 DPI 感知 / UTF-8 代码页 / 长路径，与运行时那次
//! `SetProcessDpiAwarenessContext` 一致（manifest 生效后运行时那次成了幂等空操作）。
//! Authenticode 签的是最终 PE 字节，本步在链接期嵌入，天然早于签名。

use embed_manifest::manifest::ExecutionLevel;
use embed_manifest::{embed_manifest, new_manifest};

fn main() {
    // build.rs 跑在宿主机上；只有目标是 Windows 时才嵌 manifest（mac 上交叉/本地检查时跳过）。
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let manifest = new_manifest("Qingjian.Server")
            .requested_execution_level(ExecutionLevel::AsInvoker)
            .ui_access(true);
        embed_manifest(manifest).expect("嵌入 Server uiAccess manifest 失败");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
