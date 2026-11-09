//! 把配置文件里已经下线的取值改写成现在的：只由设置程序启动时调，输入法进程只读配置、不写。

use std::path::Path;

use toml_edit::DocumentMut;

use super::{Config, ConfigError, LocalModel};

impl Config {
    /// 配置文件里写着已下线的取值就原地改成现在的，返回改没改；没有文件或没有这类取值什么都不做。
    ///
    /// 现在只有一项：`[model] name = "zhiwei"`（含章·知微已下线）改成 `tongbian`。不改写也能读（serde 按通变读），
    /// 改写是让文件里不再留着下线的名字。
    pub fn rewrite_retired_values(path: &Path) -> Result<bool, ConfigError> {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(source) => {
                return Err(ConfigError::Read {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        let document: DocumentMut = source.parse().map_err(|source| ConfigError::Edit {
            path: path.to_owned(),
            source: Box::new(source),
        })?;
        let retired = document
            .get("model")
            .and_then(|model| model.get("name"))
            .and_then(|name| name.as_str())
            == Some("zhiwei");
        if retired {
            Self::set_value(path, "model", "name", LocalModel::Tongbian.key())?;
        }
        Ok(retired)
    }
}
