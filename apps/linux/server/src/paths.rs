//! Linux XDG 配置、用户数据、日志和随包产品资源定位。
use std::path::{Path, PathBuf};

fn xdg(name: &str, fallback: &str) -> PathBuf {
    std::env::var_os(name)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(fallback)))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("qingjian")
}
pub fn user_dir() -> PathBuf {
    xdg("XDG_DATA_HOME", ".local/share")
}
pub fn config_path() -> PathBuf {
    xdg("XDG_CONFIG_HOME", ".config").join("config.toml")
}
pub fn log_dir() -> PathBuf {
    xdg("XDG_STATE_HOME", ".local/state").join("logs")
}
pub fn resource_root() -> Option<PathBuf> {
    if let Some(root) = std::env::var_os("QINGJIAN_RESOURCES") {
        return Some(root.into());
    }
    let executable = std::env::current_exe().ok()?;
    let prefix = executable.parent()?.parent()?;
    let installed = prefix.join("share/qingjian/resources");
    if installed.join("assets").is_dir() || installed.join("data").is_dir() {
        return Some(installed);
    }
    qingjian_platform::resources::bundled_root()
}
pub fn existing(path: PathBuf) -> Option<PathBuf> {
    path.is_file().then_some(path)
}
pub fn generated(root: &Path, name: &str) -> Option<PathBuf> {
    existing(root.join("data/generated").join(name))
}
pub fn asset(root: &Path, name: &str) -> Option<PathBuf> {
    existing(root.join("assets").join(name))
}
