//! 青简 Windows 设置界面入口：左侧导航栏 + 各分节表单，读写 `%APPDATA%\Qingjian\config.toml`，对齐 macOS 偏好设置。
//! UI 用 Windows Reactor（微软官方 Rust WinUI 3）。仅 Windows；其它平台编成空壳（工作区能整体编译）。
#![cfg_attr(windows, windows_subsystem = "windows")] // GUI 程序，不弹控制台窗口

#[cfg(windows)]
mod panel;

#[cfg(windows)]
fn main() {
    if let Err(error) = windows_reactor::App::run_component::<panel::Settings>(()) {
        eprintln!("设置界面启动失败: {error:?}");
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("qingjian-settings 仅支持 Windows");
}
