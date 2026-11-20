//! 找 llama-server：配置里指定了就只认它，否则依次看 PATH 与几个常见安装位置。
//!
//! 输入法进程的 PATH 往往不全：macOS 的输入法由系统拉起，PATH 里没有 Homebrew 的目录；Windows 上
//! 装 winget 包之前就在跑的进程拿不到新加的 PATH。所以另外看 Homebrew、MacPorts、`~/.local/bin`
//! 与 winget 的 llama.cpp 包目录。

use std::path::{Path, PathBuf};

/// 程序的文件名。
const PROGRAM: &str = if cfg!(windows) {
    "llama-server.exe"
} else {
    "llama-server"
};

/// 指定了路径就只看它在不在（不在不悄悄换成别处的）；没指定就到各目录里找。
pub(crate) fn locate(configured: Option<&Path>) -> Option<PathBuf> {
    match configured {
        Some(path) => path.is_file().then(|| path.to_path_buf()),
        None => search(&search_dirs()),
    }
}

/// 按顺序在这些目录里找，先找到的算。
pub(crate) fn search(dirs: &[PathBuf]) -> Option<PathBuf> {
    dirs.iter()
        .map(|dir| dir.join(PROGRAM))
        .find(|path| path.is_file())
}

/// PATH 在前，常见安装位置在后。
fn search_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default();
    if cfg!(target_os = "macos") {
        dirs.extend(["/opt/homebrew/bin", "/usr/local/bin", "/opt/local/bin"].map(PathBuf::from));
    }
    if cfg!(target_os = "linux") {
        dirs.extend(["/usr/local/bin", "/home/linuxbrew/.linuxbrew/bin"].map(PathBuf::from));
    }
    if cfg!(unix)
        && let Some(home) = std::env::var_os("HOME")
    {
        dirs.push(PathBuf::from(home).join(".local").join("bin"));
    }
    if cfg!(windows)
        && let Some(local) = std::env::var_os("LOCALAPPDATA")
    {
        let winget = PathBuf::from(local).join("Microsoft").join("WinGet");
        dirs.push(winget.join("Links"));
        // winget 把 llama.cpp 解压在 Packages\ggml.llamacpp_<来源>，程序要和旁边的 DLL 一起用，不放链接
        if let Ok(entries) = std::fs::read_dir(winget.join("Packages")) {
            dirs.extend(
                entries
                    .flatten()
                    .filter(|entry| {
                        entry
                            .file_name()
                            .to_string_lossy()
                            .starts_with("ggml.llamacpp")
                    })
                    .map(|entry| entry.path()),
            );
        }
    }
    dirs
}
