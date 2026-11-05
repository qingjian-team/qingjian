//! 青简 macOS 输入法壳（IMK）。
//!
//! 按键进 [`qingjian_core::Engine`]，候选画在自绘 NSPanel 里。数字选词，空格上首选，
//! 回车上屏拼音本身，退格删一个，Esc 清空，上下键移动高亮。
//!
//! 无法 `cargo run`：必须打包成 `.app` 装到 `~/Library/Input Methods/`，见 `scripts/bundle.sh`。
//!
//! 只在 macOS 上编；别的平台编出一个只报错退出的空壳，在仓库根目录直接 `cargo build` 也不会卡在这里。

#[cfg(target_os = "macos")]
mod app;
#[cfg(target_os = "macos")]
mod candidates;
#[cfg(target_os = "macos")]
mod error;
#[cfg(target_os = "macos")]
mod host;
#[cfg(target_os = "macos")]
mod imk;
#[cfg(target_os = "macos")]
mod launch;
#[cfg(target_os = "macos")]
mod menubar;
#[cfg(target_os = "macos")]
mod preferences;

#[cfg(target_os = "macos")]
fn main() {
    launch::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("qingjian-macos only runs on macOS");
    std::process::exit(1);
}
