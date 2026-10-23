//! 附加词库：设置里开着的随包领域词库与导入的词库挂到 Engine 上，文件或开关变了才重挂。

use std::path::Path;
use std::time::SystemTime;

use qingjian_dictionary::{Dictionary, GENERAL_DICTIONARY};

use super::Session;
use crate::options::InputOptions;
use crate::store::dictionaries::{domains_dir, files, imported_dir};

/// 挂着的一本：路径、修改时间、大小。同名重新导入时间与大小会变，据此重挂。
pub(super) type Mounted = (std::path::PathBuf, Option<SystemTime>, u64);

impl Session {
    /// 交设置时调：按开关列出该挂的文件，与上次挂的一样就不动；坏文件只记日志、跳过。
    pub(super) fn sync_dictionaries(&mut self, options: &InputOptions) {
        // 通用词库缺省开着、关掉才记进 disabled；别的领域词库缺省关着
        let domains = files(&domains_dir(&self.data_dir))
            .into_iter()
            .filter(|(id, _)| {
                if id == GENERAL_DICTIONARY {
                    !options.disabled_dictionaries.iter().any(|d| d == id)
                } else {
                    options.domains.iter().any(|d| d == id)
                }
            });
        let imported = files(&imported_dir(&self.user_dir))
            .into_iter()
            .filter(|(id, _)| !options.disabled_dictionaries.iter().any(|d| d == id));
        let wanted: Vec<Mounted> = domains
            .chain(imported)
            .map(|(_, path)| {
                let metadata = std::fs::metadata(&path).ok();
                let modified = metadata.as_ref().and_then(|m| m.modified().ok());
                let len = metadata.map_or(0, |m| m.len());
                (path, modified, len)
            })
            .collect();
        if wanted == self.dictionaries {
            return;
        }
        let loaded: Vec<Dictionary> = wanted
            .iter()
            .filter_map(|(path, ..)| {
                let mut dictionary = open(path)?;
                let general = path.file_stem().is_some_and(|s| s == GENERAL_DICTIONARY);
                dictionary.set_secondary(general);
                Some(dictionary)
            })
            .collect();
        tracing::info!(count = loaded.len(), "附加词库已挂上");
        self.engine.set_extra_dictionaries(loaded);
        self.dictionaries = wanted;
    }
}

fn open(path: &Path) -> Option<Dictionary> {
    Dictionary::from_path(path)
        .inspect_err(
            |error| tracing::warn!(%error, file = %path.display(), "附加词库加载失败，跳过"),
        )
        .ok()
}
