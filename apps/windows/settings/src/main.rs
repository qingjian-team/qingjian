//! 青简 Windows 设置界面入口：左侧导航栏 + 各分节表单，读写 `%APPDATA%\Qingjian\config.toml`。
//! UI 用 Windows Reactor；非 Windows 编成空壳，让工作区能整体编译。
#![cfg_attr(windows, windows_subsystem = "windows")]

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
