//! 青简剪贴板：常驻后台记录复制过的内容，按全局快捷键从屏幕一边弹出面板，选中后粘贴回原窗口。快捷键与方向读输入法的配置文件。
//! Windows 与 macOS。参数：`--managed`（由输入法拉起，配置里关了就退出）、`--config <配置文件>`、`--edge top|bottom|left|right`、
//! `--data <数据库文件>`、`--theme light|dark`（缺省跟系统）、`--clear`（清空历史就退出，设置程序调）、`--neon ok|fail`（亮一次同步霓虹灯就退出，调外观用）。

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(any(windows, target_os = "macos"))]
mod app;
#[cfg(any(windows, target_os = "macos"))]
mod i18n;
#[cfg(any(windows, target_os = "macos"))]
mod link;
#[cfg(any(windows, target_os = "macos"))]
mod palette;
#[cfg(any(windows, target_os = "macos"))]
mod picture;
#[cfg(any(windows, target_os = "macos"))]
mod system;
#[cfg(any(windows, target_os = "macos"))]
mod view;

/// 输入法的配置文件与剪贴板的数据库：Windows 在 `%APPDATA%` / `%LOCALAPPDATA%` 下，macOS 都在 `~/Library/Application Support/Qingjian/`。
#[cfg(windows)]
fn default_paths() -> Result<(Option<std::path::PathBuf>, std::path::PathBuf), &'static str> {
    use std::path::PathBuf;

    let config = std::env::var_os("APPDATA")
        .map(|base| PathBuf::from(base).join("Qingjian").join("config.toml"));
    let data = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or("LOCALAPPDATA is not set")?
        .join("Qingjian");
    Ok((config, data.join("clipboard").join("history.db")))
}

#[cfg(target_os = "macos")]
fn default_paths() -> Result<(Option<std::path::PathBuf>, std::path::PathBuf), &'static str> {
    use std::path::PathBuf;

    let base = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or("HOME is not set")?
        .join("Library/Application Support/Qingjian");
    Ok((
        Some(base.join("config.toml")),
        base.join("clipboard").join("history.db"),
    ))
}

/// 亮一次霓虹灯：慢、中、快三档速度各流一会儿，再按 `success` 收成绿或红。
#[cfg(any(windows, target_os = "macos"))]
fn neon_demo(success: bool) {
    let neon = system::Neon::spawn();
    neon.start();
    for speed in [500_000, 5_000_000, 50_000_000] {
        std::thread::sleep(std::time::Duration::from_millis(900));
        neon.speed(speed);
    }
    std::thread::sleep(std::time::Duration::from_millis(900));
    neon.finish(success);
    std::thread::sleep(std::time::Duration::from_millis(900));
}

#[cfg(windows)]
fn show_neon(success: bool) {
    neon_demo(success);
}

/// macOS 的灯条窗口要主线程画：主线程跑事件循环，演示放到别的线程，演完退出。
#[cfg(target_os = "macos")]
fn show_neon(success: bool) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    std::thread::spawn(move || {
        neon_demo(success);
        std::process::exit(0);
    });
    app.run();
}

#[cfg(any(windows, target_os = "macos"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::path::PathBuf;

    use bamboo_slip::{WindowOptions, run_window, system_acrylic};
    use qingjian_clipboard::{ClipSettings, ClipStore, Edge};

    let mut forced_edge = None;
    let mut forced_dark = None;
    let mut managed = false;
    let mut clear = false;
    let (mut config, mut path) = default_paths()?;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--edge" => {
                forced_edge = Some(
                    args.next()
                        .as_deref()
                        .and_then(Edge::parse)
                        .ok_or("--edge expects top, bottom, left or right")?,
                );
            }
            "--data" => {
                path = args
                    .next()
                    .map(PathBuf::from)
                    .ok_or("--data expects a path")?
            }
            "--managed" => managed = true,
            "--clear" => clear = true,
            "--neon" => {
                let success = args.next().as_deref() == Some("ok");
                show_neon(success);
                return Ok(());
            }
            "--config" => {
                config = Some(
                    args.next()
                        .map(PathBuf::from)
                        .ok_or("--config expects a path")?,
                );
            }
            "--theme" => {
                forced_dark = Some(match args.next().as_deref() {
                    Some("dark") => true,
                    Some("light") => false,
                    _ => return Err("--theme expects light or dark".into()),
                });
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }
    let settings = config
        .as_ref()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| ClipSettings::parse(&text))
        .unwrap_or_default();
    if clear {
        ClipStore::open(&path, settings.history_limit)?.clear(app::now_millis())?;
        return Ok(());
    }
    if managed && !settings.enabled {
        return Ok(());
    }
    if !system::claim_single_instance() {
        return Ok(());
    }
    bamboo_slip_i18n::set_locale(settings.locale());
    let store = ClipStore::open(&path, settings.history_limit)?;
    let acrylic = system_acrylic();
    run_window(
        app::ClipboardApp::new(
            store,
            &path,
            system::Watch {
                path: config,
                settings,
                managed,
            },
            forced_edge,
            forced_dark,
            acrylic,
        ),
        WindowOptions::new("Qingjian Clipboard", (420.0, 360.0))
            .panel()
            .acrylic(acrylic),
    )?;
    Ok(())
}

#[cfg(not(any(windows, target_os = "macos")))]
fn main() {
    eprintln!("qingjian-clipboard only supports Windows and macOS for now");
    std::process::exit(1);
}
