//! 配置文件的运行时状态：路径、当前生效的值、上次读取时的修改时间。
//!
//! 启动、菜单开关、切回输入法时的热加载都从这里拿 `Config`，壳只认这一份。
//! 解析失败不让输入法退出（输入优先于一切附加功能）：保留上一份能用的配置，把错误留给菜单显示。
//! 单条短语越界、某个分节写错只丢那一处（`Config::load_with_diagnostics`），同样在菜单里说明。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use qingjian_platform::{Config, ConfigDiagnostics};

use super::paths;

pub struct Settings {
    /// 配置文件路径；拿不到用户目录时为 `None`，此时一切按默认值、菜单开关不落盘。
    path: Option<PathBuf>,

    /// 当前生效的配置。
    config: Config,

    /// 上次成功读取时文件的修改时间，用来判断用户有没有手改过。
    modified: Option<SystemTime>,

    /// 最近一次读取失败的原因；成功后清掉。
    error: Option<String>,

    /// 上次读取里被丢掉的设置（短语、分节）；读到干净的配置就清空。
    dropped: ConfigDiagnostics,
}

impl Settings {
    /// 首次加载：先读同目录的 `.env`，文件不存在就写模板，再读配置。
    pub fn load() -> Self {
        let mut settings = Self {
            path: paths::config_file(),
            config: Config::default(),
            modified: None,
            error: None,
            dropped: ConfigDiagnostics::default(),
        };
        if let Some(path) = settings.path.clone() {
            load_dotenv(&path);
            match Config::write_template_if_missing(&path) {
                Ok(true) => tracing::info!(path = %path.display(), "已写出配置模板"),
                Ok(false) => {}
                Err(error) => tracing::warn!(%error, "写配置模板失败"),
            }
            settings.read(&path);
        }
        settings
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// 给菜单与偏好设置显示的一行提示：硬错误（整份沿用上一份）与被丢掉的条目都在这里。
    /// 没有问题时返回 `None`。
    pub fn notice(&self) -> Option<String> {
        let mut parts = Vec::new();
        if let Some(error) = &self.error {
            parts.push(format!("配置文件解析失败，已沿用上一份：{error}"));
        }
        if !self.dropped.is_empty() {
            parts.push(format!("已忽略：{}", self.dropped));
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("；"))
        }
    }

    /// 文件的修改时间和上次读的不一样就重读；返回是否重读了（重读失败也算，配置没变但错误状态变了）。
    pub fn reload_if_changed(&mut self) -> bool {
        let Some(path) = self.path.clone() else {
            return false;
        };
        if modified_time(&path) == self.modified {
            return false;
        }
        self.reload();
        true
    }

    /// 强制重读，`.env` 也再读一遍（不覆盖已有的环境变量，只补新加的）。
    pub fn reload(&mut self) {
        if let Some(path) = self.path.clone() {
            load_dotenv(&path);
            self.read(&path);
        }
    }

    /// 原地改一个布尔键并重读，见 [`Self::set_value`]。
    pub fn set_bool(&mut self, section: &str, key: &str, value: bool) -> bool {
        self.set_value(section, key, value)
    }

    /// 原地改一个键并重读。写失败只记日志，返回 `false`。
    pub fn set_value(
        &mut self,
        section: &str,
        key: &str,
        value: impl Into<toml_edit::Value>,
    ) -> bool {
        let Some(path) = self.path.clone() else {
            tracing::warn!("没有配置文件路径，设置不落盘");
            return false;
        };
        let value = value.into();
        if let Err(error) = Config::set_value(&path, section, key, value.clone()) {
            tracing::warn!(%error, section, key, "写配置失败");
            return false;
        }
        tracing::info!(section, key, value = %value.to_string().trim(), "配置已改");
        self.read(&path);
        true
    }

    /// 把一个环境变量（密钥）写进配置同目录的 `.env` 并立即注入当前进程。
    /// 文件仅本用户可读；值不进日志。返回是否成功。
    pub fn set_env_var(&self, name: &str, value: &str) -> bool {
        let Some(path) = &self.path else {
            tracing::warn!("没有配置目录，密钥无处可存");
            return false;
        };
        // 先登记再动文件：后面哪一步失败，日志里都不会出现这个值
        qingjian_platform::logs::secrets::register(value);
        let env_file = path.with_file_name(".env");
        let existing = std::fs::read_to_string(&env_file).unwrap_or_default();
        let prefix = format!("{name}=");
        let mut lines: Vec<&str> = existing
            .lines()
            .filter(|line| !line.trim_start().starts_with(&prefix))
            .collect();
        let entry = format!("{prefix}{value}");
        lines.push(&entry);
        let content = format!("{}\n", lines.join("\n"));
        // 原子写且仅本用户可读（0600）
        let written = qingjian_core::storage::write_atomic_private(&env_file, |file| {
            file.write_all(content.as_bytes())
        });
        if let Err(error) = written {
            tracing::warn!(%error, path = %env_file.display(), "写 .env 失败");
            return false;
        }
        // 解析错误的原文里带着出错的那一行（含密钥），不进日志
        if dotenvy::from_path_override(&env_file).is_err() {
            tracing::warn!(path = %env_file.display(), ".env 写入后重新加载失败");
            return false;
        }
        tracing::info!(name, path = %env_file.display(), "密钥已写入 .env");
        true
    }

    fn read(&mut self, path: &Path) {
        match Config::load_with_diagnostics(path) {
            Ok((config, dropped)) => {
                tracing::info!(
                    path = %path.display(),
                    predict = config.predict.enabled,
                    fuzzy = config.fuzzy.any(),
                    dropped = dropped.len(),
                    "配置已加载"
                );
                if !dropped.is_empty() {
                    tracing::warn!(%dropped, "配置里有条目被忽略，其余设置照常生效");
                }
                self.config = config;
                self.dropped = dropped;
                self.error = None;
            }
            Err(error) => {
                tracing::warn!(%error, "配置读取失败，沿用上一份");
                self.error = Some(error.to_string());
                self.dropped = ConfigDiagnostics::default();
            }
        }
        // 失败也记时间：同一份坏文件不用每次激活都重读一遍
        self.modified = modified_time(path);
    }
}

/// 输入法进程由 launchd 拉起，看不到 shell 的环境变量：配置同目录的 `.env`（如 `QINGJIAN_API_KEY=...`）先读进环境。
fn load_dotenv(config_path: &Path) {
    let env_file = config_path.with_file_name(".env");
    match dotenvy::from_path(&env_file) {
        Ok(()) => tracing::info!(path = %env_file.display(), "已加载 .env"),
        Err(dotenvy::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {}
        // 解析错误的原文里带着出错的那一行（含密钥），只记是哪一类错
        Err(dotenvy::Error::LineParse(_, position)) => {
            tracing::warn!(path = %env_file.display(), position, ".env 有一行格式不对，整个文件没读进来");
        }
        Err(dotenvy::Error::Io(error)) => {
            tracing::warn!(path = %env_file.display(), %error, ".env 读取失败");
        }
        Err(_) => tracing::warn!(path = %env_file.display(), ".env 读取失败"),
    }
}

fn modified_time(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}
