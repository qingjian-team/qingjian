//! 各平台的用户目录：配置、数据、主题、日志放在哪，与各平台输入法读写的位置一致。
//! Windows 用 `qingjian_platform::dirs`；macOS 照 `apps/macos/src/app/paths.rs`，Linux 照 `apps/linux/server/src/paths.rs`。

use std::path::PathBuf;

/// 用户数据目录：学习数据、统计、输入日志都在这里。
pub fn user_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        qingjian_platform::dirs::user_dir()
    } else if cfg!(target_os = "macos") {
        home().map(|home| home.join("Library/Application Support/Qingjian"))
    } else {
        xdg("XDG_DATA_HOME", ".local/share")
    }
}

pub fn config_path() -> Option<PathBuf> {
    if cfg!(any(windows, target_os = "macos")) {
        user_dir().map(|dir| dir.join("config.toml"))
    } else {
        xdg("XDG_CONFIG_HOME", ".config").map(|dir| dir.join("config.toml"))
    }
}

pub fn themes_dir() -> Option<PathBuf> {
    user_dir().map(|dir| dir.join("themes"))
}

/// 输入法的运行日志目录。
pub fn log_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        qingjian_platform::dirs::log_dir()
    } else if cfg!(target_os = "macos") {
        home().map(|home| home.join("Library/Logs/Qingjian"))
    } else {
        xdg("XDG_STATE_HOME", ".local/state").map(|dir| dir.join("logs"))
    }
}

/// 输入日志（每次上屏一行）。
pub fn input_log() -> Option<PathBuf> {
    user_dir().map(|dir| dir.join("input-log.jsonl"))
}

/// macOS 上找随包数据的几个 `Resources` 目录，按先后试：自己所在的 .app 往外每一层（设置程序装在
/// `Qingjian.app/Contents/Applications/` 里，数据在外层），再是装好的输入法。别的平台为空，走 `bundled_resource`。
pub fn mac_resources() -> Vec<PathBuf> {
    if !cfg!(target_os = "macos") {
        return Vec::new();
    }
    let mut dirs: Vec<PathBuf> = std::env::current_exe()
        .ok()
        .map(|exe| {
            exe.ancestors()
                .filter(|dir| dir.extension().is_some_and(|ext| ext == "app"))
                .map(|app| app.join("Contents/Resources"))
                .collect()
        })
        .unwrap_or_default();
    let installed = "Library/Input Methods/Qingjian.app/Contents/Resources";
    if let Some(home) = home() {
        dirs.push(home.join(installed));
    }
    dirs.push(PathBuf::from("/").join(installed));
    dirs
}

/// 随包资源（相对随包根，如 `data/generated/dicts`），找法同 Linux Server：`QINGJIAN_RESOURCES`，
/// 再 `<前缀>/share/qingjian/resources`（Linux 装机布局），再 exe 同级或开发布局（`qingjian_platform::resources`）。
pub fn bundled(rel: &str) -> Option<PathBuf> {
    let installed = std::env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.parent()?.parent()?.join("share/qingjian/resources")));
    std::env::var_os("QINGJIAN_RESOURCES")
        .map(PathBuf::from)
        .into_iter()
        .chain(installed)
        .map(|root| root.join(rel))
        .find(|path| path.exists())
        .or_else(|| qingjian_platform::resources::bundled_resource(rel))
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// XDG 目录下的 `qingjian`：环境变量不是绝对路径时按规范回到主目录下的缺省位置。
fn xdg(name: &str, fallback: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| home().map(|home| home.join(fallback)))
        .map(|dir| dir.join("qingjian"))
}
