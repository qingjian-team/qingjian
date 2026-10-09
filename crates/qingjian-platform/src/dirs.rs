//! Windows 上用户目录的约定，Server / TSF DLL / 设置程序三处共用：
//!
//! - **数据** `%APPDATA%\Qingjian`：配置、密钥、学习数据、统计、输入日志（漫游目录，跟着账户走）。
//! - **日志** `%LOCALAPPDATA%\Qingjian\logs`：三个进程的运行日志都在这一个目录里，
//!   `server.<日期>.log` / `tsf.<日期>.log` / `settings.<日期>.log`，用户反馈问题时整个目录打包即可。
//!   放本机目录有两个原因：日志本来就是这台机器的东西，不该跟账户漫游；DLL 被加载进
//!   AppContainer 应用（任务栏搜索 / 设置）时写不了漫游目录，Server 启动时会给这个目录授权。
//!
//! 只查环境变量、不碰系统 API，其他平台也能编（拿不到为 `None`）。macOS 有自己的 `paths.rs`，不走这里。

use std::path::PathBuf;

/// 用户数据目录 `%APPDATA%\Qingjian`。
pub fn user_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|base| PathBuf::from(base).join("Qingjian"))
}

/// 配置文件 `%APPDATA%\Qingjian\config.toml`。
pub fn config_path() -> Option<PathBuf> {
    user_dir().map(|dir| dir.join("config.toml"))
}

/// 用户主题目录 `%APPDATA%\Qingjian\themes`，一个主题一个子目录（`<id>\theme.json`），不负责创建。
pub fn themes_dir() -> Option<PathBuf> {
    user_dir().map(|dir| dir.join("themes"))
}

/// 运行日志目录 `%LOCALAPPDATA%\Qingjian\logs`，不负责创建。
pub fn log_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|base| PathBuf::from(base).join("Qingjian").join("logs"))
}
