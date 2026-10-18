//! 青简 Windows 设置界面入口：左侧导航栏 + 各分节表单，读写 `%APPDATA%\Qingjian\config.toml`。
//! UI 用 Windows Reactor；非 Windows 编成空壳，让工作区能整体编译。
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod log;
#[cfg(windows)]
mod panel;
#[cfg(windows)]
mod single_instance;

#[cfg(windows)]
fn main() {
    if !single_instance::acquire() {
        return;
    }
    if let Err(error) = windows_reactor::App::run_component::<panel::Settings>(()) {
        // GUI 子系统没有控制台：记文件日志，再弹个框让用户知道发生了什么（最常见是运行库文件缺失）。
        log::error(format!("设置界面启动失败: {error:?}"));
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("青简旧版设置")
            .set_description(format!(
                "设置界面启动失败，请重新安装青简；仍不行请把日志目录发给作者。\n\n{error}"
            ))
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("qingjian-settings 仅支持 Windows");
}
