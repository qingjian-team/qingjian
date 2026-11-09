//! 按钮触发的一次性动作：打开文件或目录、打包日志、清空输入日志、打开网页。

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::i18n::{tr, tr_args};
use crate::system::{paths, system_info};

pub const WEBSITE_URL: &str = "https://qingjian.app";

pub const REPOSITORY_URL: &str = "https://github.com/qingjian-team";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// 用文本编辑器打开配置文件。
    OpenConfig,
    OpenDataDir,
    OpenLogDir,
    /// 日志目录加配置文件打成桌面上的 zip。
    ExportLogs,
    /// 第一次点只是让按钮变成「再点一次清空」，见 [`crate::settings::Settings::armed`]。
    ClearInputLog,
    /// 清空剪贴板历史，同样点两下。
    ClearClipboard,
    OpenWebsite,
    OpenRepository,
    /// 官网的下载页。
    OpenDownload,
}

impl Action {
    /// 做完的那句话显示在哪一行下面，见 [`crate::settings::Settings::notice`]。
    pub fn slot(self) -> &'static str {
        match self {
            Self::OpenLogDir | Self::ExportLogs => "logs",
            Self::ClearInputLog => "input-log",
            Self::ClearClipboard => "clipboard-clear",
            Self::OpenConfig | Self::OpenDataDir => "config",
            Self::OpenWebsite | Self::OpenRepository => "links",
            Self::OpenDownload => "update",
        }
    }

    /// 做完给用户看的一句话；`None` 是没什么可说的（打开窗口这类，看得见结果）。
    pub fn run(self, config: &Path) -> Option<String> {
        let result = match self {
            Self::OpenConfig => open_in_editor(config),
            Self::OpenDataDir => open_dir(paths::user_dir()),
            Self::OpenLogDir => open_dir(paths::log_dir()),
            Self::ExportLogs => export_logs(config).map(|zip| {
                reveal(&zip);
                tr("action.logs_exported").to_owned()
            }),
            Self::ClearInputLog => {
                clear_input_log().map(|()| tr("action.input_log_cleared").to_owned())
            }
            Self::ClearClipboard => {
                clear_clipboard().map(|()| tr("action.clipboard_cleared").to_owned())
            }
            Self::OpenWebsite => open(WEBSITE_URL).map(|()| String::new()),
            Self::OpenRepository => open(REPOSITORY_URL).map(|()| String::new()),
            Self::OpenDownload => open(qingjian_update::DOWNLOAD_URL).map(|()| String::new()),
        };
        match result {
            Ok(message) => (!message.is_empty()).then_some(message),
            Err(error) => {
                tracing::warn!(?self, %error, "动作失败");
                Some(error)
            }
        }
    }
}

fn open_in_editor(path: &Path) -> Result<String, String> {
    if !path.exists() {
        qingjian_platform::Config::write_template_if_missing(path)
            .map_err(|error| tr_args("action.cant_create_config", &[("error", &error)]))?;
    }
    let path = path.to_string_lossy();
    if cfg!(windows) {
        spawn("notepad.exe", &[&path])?;
    } else if cfg!(target_os = "macos") {
        spawn("/usr/bin/open", &["-t", &path])?;
    } else {
        spawn("xdg-open", &[&path])?;
    }
    Ok(String::new())
}

fn open_dir(dir: Option<PathBuf>) -> Result<String, String> {
    let dir = dir.ok_or(tr("action.no_dir"))?;
    std::fs::create_dir_all(&dir)
        .map_err(|error| tr_args("action.cant_create_dir", &[("error", &error)]))?;
    open(&dir.to_string_lossy())?;
    Ok(String::new())
}

/// 交给系统打开（目录、网址都行）。
pub fn open(target: &str) -> Result<(), String> {
    if cfg!(windows) {
        spawn("explorer.exe", &[target])
    } else if cfg!(target_os = "macos") {
        spawn("/usr/bin/open", &[target])
    } else {
        spawn("xdg-open", &[target])
    }
}

/// 在文件管理器里选中这个文件；做不到也无妨，zip 已经在桌面上了。
fn reveal(path: &Path) {
    let path = path.to_string_lossy();
    let _ = if cfg!(windows) {
        spawn("explorer.exe", &[&format!("/select,{path}")])
    } else if cfg!(target_os = "macos") {
        spawn("/usr/bin/open", &["-R", &path])
    } else {
        Ok(())
    };
}

fn spawn(program: &str, args: &[&str]) -> Result<(), String> {
    Command::new(program)
        .args(args)
        .spawn()
        .map(drop)
        .map_err(|error| {
            tr_args(
                "action.cant_launch",
                &[("program", &program), ("error", &error)],
            )
        })
}

/// 日志目录里的文件加配置文件与系统信息（[`system_info::report`]）打成桌面上的 `qingjian-logs-<日期>.zip`（不带目录层级）。
/// 配置文件里可能写着云服务密钥，打包的是抹掉密钥的副本。
fn export_logs(config: &Path) -> Result<PathBuf, String> {
    let dir = paths::log_dir().ok_or(tr("action.no_log_dir"))?;
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|error| tr_args("action.cant_read_logs", &[("error", &error)]))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_file())
        .collect();
    if files.is_empty() {
        return Err(tr("action.no_logs").to_owned());
    }
    let staging = std::env::temp_dir().join("qingjian-export");
    std::fs::create_dir_all(&staging)
        .map_err(|error| tr_args("action.cant_copy_config", &[("error", &error)]))?;
    if let Ok(text) = std::fs::read_to_string(config) {
        let copy = staging.join("config.toml");
        std::fs::write(&copy, redact_secrets(&text))
            .map_err(|error| tr_args("action.cant_copy_config", &[("error", &error)]))?;
        files.push(copy);
    }
    // 系统版本这些取不到就少一份，不挡打包
    let system = staging.join("system.txt");
    if std::fs::write(&system, system_info::report()).is_ok() {
        files.push(system);
    }
    let desktop = desktop().ok_or(tr("action.no_desktop"))?;
    let zip = desktop.join(format!("qingjian-logs-{}.zip", today()));
    let _ = std::fs::remove_file(&zip);
    // Windows 10 起自带 bsdtar，`-a` 按扩展名出 zip；macOS 与 Linux 用 zip
    let status = if cfg!(windows) {
        let mut command = Command::new("tar.exe");
        command.arg("-a").arg("-cf").arg(&zip);
        for file in &files {
            if let (Some(parent), Some(name)) = (file.parent(), file.file_name()) {
                command.arg("-C").arg(parent).arg(name);
            }
        }
        command.status()
    } else {
        Command::new("zip")
            .arg("-jq")
            .arg(&zip)
            .args(&files)
            .status()
    }
    .map_err(|error| tr_args("action.cant_zip", &[("error", &error)]))?;
    if !status.success() {
        return Err(tr_args("action.zip_failed", &[("status", &status)]));
    }
    tracing::warn!(zip = %zip.display(), "用户导出日志");
    Ok(zip)
}

/// `api_key = "…"` 这样的行抹掉值；`api_key_env` 写的是变量名，不抹。
fn redact_secrets(text: &str) -> String {
    text.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("api_key") && !trimmed.starts_with("api_key_env") {
                "api_key = \"<已抹掉>\"".to_owned()
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn desktop() -> Option<PathBuf> {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })?;
    Some(PathBuf::from(home).join("Desktop"))
}

fn today() -> String {
    jiff::Zoned::now().strftime("%Y-%m-%d").to_string()
}

/// 剪贴板程序：Windows 与设置程序同目录；macOS 是输入法包里另一个 .app，与设置程序的 .app 同在 `Contents/Applications/`。
#[cfg(not(target_os = "macos"))]
fn clipboard_program(settings: &std::path::Path) -> std::path::PathBuf {
    settings.with_file_name("qingjian-clipboard.exe")
}

#[cfg(target_os = "macos")]
fn clipboard_program(settings: &std::path::Path) -> std::path::PathBuf {
    // …/Contents/Applications/QingjianSettings.app/Contents/MacOS/qingjian-settings
    let applications = settings.ancestors().nth(4).unwrap_or(settings);
    applications.join("QingjianClipboard.app/Contents/MacOS/qingjian-clipboard")
}

/// 历史归剪贴板程序管，让它来清；它开着也没关系，SQLite 自己协调。
fn clear_clipboard() -> Result<(), String> {
    let settings = std::env::current_exe()
        .map_err(|error| tr_args("action.clear_failed", &[("error", &error)]))?;
    let program = clipboard_program(&settings);
    let status = Command::new(program)
        .arg("--clear")
        .status()
        .map_err(|error| tr_args("action.clear_failed", &[("error", &error)]))?;
    if status.success() {
        Ok(())
    } else {
        Err(tr_args("action.clear_failed", &[("error", &status)]))
    }
}

/// 输入法开着输入日志时一直以追加方式写它，截断后接着从头写，不用通知输入法。
fn clear_input_log() -> Result<(), String> {
    let path = paths::input_log().ok_or(tr("common.no_data_dir"))?;
    if !path.exists() {
        return Ok(());
    }
    std::fs::File::create(&path)
        .map(drop)
        .map_err(|error| tr_args("action.clear_failed", &[("error", &error)]))
}
