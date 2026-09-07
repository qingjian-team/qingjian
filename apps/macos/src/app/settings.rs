//! 配置文件的运行时状态：路径、当前生效的值、上次读取时的修改时间。
//!
//! 启动、菜单开关、切回输入法时的热加载都从这里拿 `Config`，壳只认这一份。
//! 解析失败不让输入法退出（输入优先于一切附加功能）：保留上一份能用的配置，把错误留给菜单显示。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use qingjian_platform::Config;

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
}

impl Settings {
    /// 首次加载：先读同目录的 `.env`，文件不存在就写模板，再读配置。
    pub fn load() -> Self {
        let mut settings = Self {
            path: paths::config_file(),
            config: Config::default(),
            modified: None,
            error: None,
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

    /// 最近一次读取的错误文案，给菜单显示。
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
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
        if let Err(error) = dotenvy::from_path_override(&env_file) {
            tracing::warn!(%error, ".env 写入后重新加载失败");
            return false;
        }
        tracing::info!(name, path = %env_file.display(), "密钥已写入 .env");
        true
    }

    fn read(&mut self, path: &Path) {
        match Config::load(path) {
            Ok(config) => {
                tracing::info!(
                    path = %path.display(),
                    predict = config.predict.enabled,
                    fuzzy = config.fuzzy.any(),
                    "配置已加载"
                );
                self.config = config;
                self.error = None;
            }
            Err(error) => {
                tracing::warn!(%error, "配置读取失败，沿用上一份");
                self.error = Some(error.to_string());
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
        Err(error) => tracing::warn!(path = %env_file.display(), %error, ".env 读取失败"),
    }
}

fn modified_time(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}
