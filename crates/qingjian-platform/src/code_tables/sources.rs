//! 辅码码表从哪几处找：老位置 `codes/` 与方案目录 `schemes/`。启动与热加载用同一份，免得两边找的地方不一样。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use qingjian_dictionary::{AuxCodeLookup, AuxCodeTable};

use super::{AuxSource, STROKE, stroke_word_rules};
use crate::AuxCodeConfig;
use crate::aux_schemes;
use crate::code_schemes::DIR;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuxCodeSources {
    /// 随包的 `.qj` 码表目录。
    pub bundled_codes: Option<PathBuf>,

    /// 用户导入的码表目录（`<用户目录>/codes`）。
    pub user_codes: Option<PathBuf>,

    /// 随包的 `schemes` 目录。
    pub bundled_schemes: Option<PathBuf>,

    /// 用户数据根目录，方案在它的 `schemes/` 下。
    pub user_dir: Option<PathBuf>,
}

impl AuxCodeSources {
    /// 可选的几套，按先后：随包的 `.qj`、导入的 `.qj`、辅码方案（随包、用户）。id 重了的只留前面那套。
    pub fn options(&self) -> Vec<(String, AuxSource)> {
        let mut options: Vec<(String, AuxSource)> = Vec::new();
        for dir in [&self.bundled_codes, &self.user_codes]
            .into_iter()
            .flatten()
        {
            for (stem, path) in super::list(dir) {
                options.push((stem, AuxSource::File(path)));
            }
        }
        for scheme in aux_schemes::list(self.user_dir.as_deref(), self.bundled_schemes.as_deref()) {
            options.push((scheme.id.clone(), AuxSource::Scheme(scheme)));
        }
        let mut seen = Vec::new();
        options.retain(|(id, _)| {
            let fresh = !seen.contains(id);
            seen.push(id.clone());
            fresh
        });
        options
    }

    /// 按 `[aux_code] scheme` 挑一套装上；没有可用的、或那套读不了时为空（辅码不触发）。
    /// 返回列表是 Engine 的接口形状，壳这边至多一张。
    pub fn load(&self, config: &AuxCodeConfig) -> Vec<Arc<dyn AuxCodeLookup>> {
        let options = self.options();
        let Some(id) = config.pick(options.iter().map(|(id, _)| id.as_str())) else {
            return Vec::new();
        };
        if !config.scheme.trim().is_empty() && config.scheme.trim() != id {
            tracing::warn!(chosen = %config.scheme, fallback = id, "选的辅码找不到，改用这一套");
        }
        let Some((_, source)) = options.iter().find(|(option, _)| option == id) else {
            return Vec::new();
        };
        let loaded = match source {
            AuxSource::File(path) if id == STROKE && self.is_bundled(path) => {
                AuxCodeTable::open(path).map(|table| table.with_word_rules(stroke_word_rules()))
            }
            AuxSource::File(path) => AuxCodeTable::open(path),
            AuxSource::Scheme(scheme) => AuxCodeTable::from_path(&scheme.table)
                .map(|table| table.with_word_rules(scheme.words.clone())),
        };
        match loaded {
            Ok(table) => {
                tracing::info!(id, entries = table.len(), "辅码码表已加载");
                vec![Arc::new(table)]
            }
            Err(error) => {
                tracing::warn!(id, %error, "辅码码表读不了，辅码不生效");
                Vec::new()
            }
        }
    }

    fn is_bundled(&self, path: &std::path::Path) -> bool {
        self.bundled_codes
            .as_deref()
            .is_some_and(|dir| path.parent() == Some(dir))
    }

    /// 用户这边码表的快照（路径、mtime、长度）：`codes/` 里的 `.qj` 与 `schemes/` 下每个方案目录里的文件。
    /// 不解析 `scheme.toml`，轮询时只 stat；半拷进来的文件写完长度才稳定，就地改写也能被发现。
    pub fn snapshot(&self) -> Vec<(PathBuf, Option<SystemTime>, u64)> {
        let mut files: Vec<PathBuf> = self
            .user_codes
            .as_deref()
            .map(|dir| super::list(dir).into_iter().map(|(_, path)| path).collect())
            .unwrap_or_default();
        if let Some(schemes) = self.user_dir.as_ref().map(|dir| dir.join(DIR)) {
            files.extend(
                read_dir(&schemes)
                    .filter(|path| path.is_dir())
                    .flat_map(|dir| read_dir(&dir).filter(|path| path.is_file())),
            );
        }
        files.sort();
        files
            .into_iter()
            .filter_map(|path| {
                let metadata = std::fs::metadata(&path).ok()?;
                Some((path, metadata.modified().ok(), metadata.len()))
            })
            .collect()
    }
}

fn read_dir(dir: &std::path::Path) -> impl Iterator<Item = PathBuf> + use<> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
}
