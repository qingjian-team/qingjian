//! 青简剪贴板：常驻后台记录复制过的内容，按全局快捷键从屏幕一边弹出面板，选中后粘贴回原窗口。快捷键与方向读输入法的配置文件。
//! 原型阶段只有 Windows。参数：`--managed`（由输入法拉起，配置里关了就退出）、`--config <配置文件>`、`--edge top|bottom|left|right`、
//! `--data <数据库文件>`、`--theme light|dark`（缺省跟系统）、`--clear`（清空历史就退出，设置程序调）。

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(windows)]
mod app;
#[cfg(windows)]
mod i18n;
#[cfg(windows)]
mod palette;
#[cfg(windows)]
mod picture;
#[cfg(windows)]
mod system;
#[cfg(windows)]
mod view;

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::path::PathBuf;

    use bamboo_slip::{WindowOptions, run_window, system_acrylic};
    use qingjian_clipboard::{ClipSettings, ClipStore, Edge};

    let mut forced_edge = None;
    let mut forced_dark = None;
    let mut managed = false;
    let mut clear = false;
    let mut config = std::env::var_os("APPDATA")
        .map(|base| PathBuf::from(base).join("Qingjian").join("config.toml"));
    let mut path = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or("LOCALAPPDATA is not set")?
        .join("Qingjian")
        .join("clipboard")
        .join("history.db");
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

#[cfg(not(windows))]
fn main() {
    eprintln!("qingjian-clipboard only supports Windows for now");
    std::process::exit(1);
}
