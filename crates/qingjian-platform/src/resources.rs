//! 随包资源（词库 / 语言模型 / 释义表 / 等级表 / emoji / 样例）的定位。
//!
//! 两套布局，装机优先、回落开发：
//! - **装机**：资源与可执行文件同级（安装程序把 `data\` / `assets\` 装在 exe 旁）。
//! - **开发**：仓库 `ime/` 目录，exe 在 `ime\target\{debug,release}\` 下，往上三层。
//!
//! 相对写法两套布局一致（如 `data/generated/dict.qj`、`assets/levels`），只有根不同。
//! Linux 多一套**系统布局**：发行版包把数据装进 `/usr/share/qingjian`，可执行文件在 `/usr/bin`，两者不同级。
//! Windows 的 Server 进程与设置窗口共用；macOS 有自己的 `paths.rs`，不走这里。

use std::path::{Path, PathBuf};

/// Linux 发行版包的数据位置，按先后顺序找：本地装的优先于发行版装的，最后才是 Flatpak。
///
/// `/app` 是 Flatpak 给应用的前缀（沙箱里的 `/usr` 是 runtime 的，不会有我们的东西）。
#[cfg(target_os = "linux")]
const SYSTEM_ROOTS: [&str; 3] = [
    "/usr/local/share/qingjian",
    "/usr/share/qingjian",
    "/app/share/qingjian",
];

/// 随包资源的根目录：其下有 `data/` 与 `assets/`。装机布局与 exe 同级，否则回落开发布局的仓库根，
/// Linux 上再回落系统布局（`/usr/share/qingjian`）；都没有为 `None`。
pub fn bundled_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let exe_dir = exe.parent()?;
    if has_resources(exe_dir) {
        return Some(exe_dir.to_path_buf());
    }
    if let Some(dev_root) = dev_root(&exe)
        && has_resources(dev_root)
    {
        return Some(dev_root.to_path_buf());
    }
    system_root()
}

/// 开发布局的仓库根：`exe → {debug,release} → target → 仓库根`。
///
/// **必须先确认 exe 真的在 cargo 的 target 目录里**，不能只往上数三层：装到 `/usr/bin` 之后
/// 往上三层正好是 `/`，只要机器根目录下有个 `/data` 或 `/assets`（真机上见过 `/data`），
/// 就会把 `/` 当成随包根，然后静默回落到几十条的样例词库——症状是「装好了但一个词都打不出」。
fn dev_root(exe: &Path) -> Option<&Path> {
    let profile = exe.parent()?;
    let name = profile.file_name()?.to_str()?;
    if name != "debug" && name != "release" {
        return None;
    }
    let target = profile.parent()?;
    (target.file_name()?.to_str()? == "target").then(|| target.parent())?
}

/// 系统布局的根（只有 Linux 有）：发行版包把数据装在这里，而可执行文件在 `/usr/bin`，不与它同级。
#[cfg(target_os = "linux")]
fn system_root() -> Option<PathBuf> {
    SYSTEM_ROOTS
        .iter()
        .map(Path::new)
        .find(|dir| has_resources(dir))
        .map(Path::to_path_buf)
}

/// 别的平台没有系统布局：装机布局与开发布局之外就是没有。
#[cfg(not(target_os = "linux"))]
fn system_root() -> Option<PathBuf> {
    None
}

/// 一个随包资源的完整路径（相对随包根，如 `data/generated/dict.qj`）；根找不到或该路径不存在为 `None`。
pub fn bundled_resource(rel: &str) -> Option<PathBuf> {
    let path = bundled_root()?.join(rel);
    path.exists().then_some(path)
}

/// 一个目录是不是随包根：有 `data` 或 `assets` 子目录就算。
fn has_resources(dir: &Path) -> bool {
    dir.join("data").is_dir() || dir.join("assets").is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 装到 `/usr/bin` 的可执行文件不能把 `/` 当成仓库根——根目录下有 `/data` 的机器上真踩过。
    #[test]
    fn installed_binary_has_no_dev_root() {
        assert_eq!(dev_root(Path::new("/usr/bin/qingjian-linux")), None);
        assert_eq!(dev_root(Path::new("/usr/local/bin/qingjian-linux")), None);
    }

    /// 开发布局照常认出来。
    #[test]
    fn cargo_target_layout_is_recognised() {
        assert_eq!(
            dev_root(Path::new("/home/me/repo/target/debug/qingjian-linux")),
            Some(Path::new("/home/me/repo"))
        );
        assert_eq!(
            dev_root(Path::new("/home/me/repo/target/release/qingjian-linux")),
            Some(Path::new("/home/me/repo"))
        );
    }

    /// 目录名对不上就不认：别的地方叫 `release` 的目录不算。
    #[test]
    fn only_a_real_cargo_target_counts() {
        assert_eq!(dev_root(Path::new("/opt/app/release/qingjian-linux")), None);
        assert_eq!(
            dev_root(Path::new("/opt/target/other/qingjian-linux")),
            None
        );
    }
}
