//! Server 严格读取配置，缺文件不能伪装成已加载默认配置；错误日志只输出类别。

use std::path::Path;

use qingjian_platform::{Config, ConfigError};

#[cfg(test)]
mod tests;

pub fn read(path: &Path) -> Result<Config, ConfigError> {
    let source = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_owned(),
        source,
    })?;
    let config: Config = toml::from_str(&source).map_err(|source| ConfigError::Parse {
        path: path.to_owned(),
        source: Box::new(source),
    })?;
    if let Some(key) = config.predict.resolve_api_key() {
        qingjian_platform::logs::secrets::register(&key);
    }
    Ok(config)
}

pub fn failure_reason(error: &ConfigError) -> &'static str {
    match error {
        ConfigError::Read { source, .. } if source.kind() == std::io::ErrorKind::NotFound => {
            "missing_file"
        }
        ConfigError::Read { .. } => "read_failed",
        ConfigError::Parse { .. } => "parse_failed",
        _ => "config_failed",
    }
}
