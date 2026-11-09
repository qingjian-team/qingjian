//! 升级后补下词库：老版本随包带着全部领域词库、通用扩展词库和德、西、日、越释义表，新版只随包基础几本，其余改从词库仓库下载。
//! 老用户开着的那几本在设置程序打开时排进下载队列依次下回来，升级前后候选不变。要下哪些记在 `downloads/restore.txt`
//! （见 `UserDirs::restore_list`）：文件不在时按配置算一次写进去，下好一本划掉一本。

use std::path::PathBuf;

use qingjian_core::Language;
use qingjian_dictionary::GENERAL_DICTIONARY;
use qingjian_platform::Config;

use super::dict_index::DictEntry;
use crate::library::Library;

#[derive(Debug, Default)]
pub struct Restore {
    /// 还没下好的 id；`None` 是还没读过清单文件（要等词库清单到手）。
    pending: Option<Vec<String>>,
}

impl Restore {
    /// 还要下的：清单里还没装、词库清单里也有的，按清单顺序。第一次调用时读（或算出并写下）要补下的清单。
    pub fn pending(
        &mut self,
        config: &Config,
        library: &Library,
        index: &[DictEntry],
    ) -> Vec<DictEntry> {
        if index.is_empty() {
            return Vec::new();
        }
        let pending = self.pending.get_or_insert_with(|| load(config, library));
        pending.retain(|id| !installed(library, id));
        pending
            .iter()
            .filter_map(|id| index.iter().find(|entry| entry.id == *id).cloned())
            .collect()
    }

    /// 下好了一本：从清单里划掉。不在清单里的（用户自己点的下载）不管。
    pub fn done(&mut self, id: &str) {
        let Some(pending) = self.pending.as_mut() else {
            return;
        };
        if !pending.iter().any(|item| item == id) {
            return;
        }
        pending.retain(|item| item != id);
        save(pending);
    }
}

fn path() -> Option<PathBuf> {
    crate::system::paths::user_dirs().map(|dirs| dirs.restore_list())
}

/// 读清单文件；不在就按配置算一份写下。没有数据目录时不补下。
fn load(config: &Config, library: &Library) -> Vec<String> {
    let Some(path) = path() else {
        return Vec::new();
    };
    if let Ok(text) = std::fs::read_to_string(&path) {
        return text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect();
    }
    let ids = wanted(config, library);
    tracing::info!(ids = ?ids, "dictionaries to restore after upgrade");
    save(&ids);
    ids
}

fn save(ids: &[String]) {
    let Some(path) = path() else {
        return;
    };
    let text: String = ids.iter().map(|id| format!("{id}\n")).collect();
    if let Err(error) = std::fs::write(&path, text) {
        tracing::warn!(path = %path.display(), %error, "restore list not saved");
    }
}

/// 老用户该补下的：配置里开着、新版不随包的领域词库，没关掉的通用扩展词库，学习语言缺的释义表。
fn wanted(config: &Config, library: &Library) -> Vec<String> {
    let dictionaries = &config.dictionaries;
    let general = dictionaries
        .is_enabled(GENERAL_DICTIONARY)
        .then_some(GENERAL_DICTIONARY);
    let mut ids: Vec<String> = dictionaries
        .domains
        .iter()
        .map(String::as_str)
        .chain(general)
        .filter(|stem| !library.domains.iter().any(|entry| entry.stem == *stem))
        .map(dict_id)
        .collect();
    let general = &config.general;
    if !general.learning_language_off()
        && let Ok(language) = general.learning_language.parse::<Language>()
        && !library.glossary_languages.contains(&language)
    {
        ids.push(format!("glossary-{}", language.code()));
    }
    ids.retain(|id| !installed(library, id));
    ids.dedup();
    ids
}

/// 领域词库在老版本里按文件名开关（`historical_figures`），词库仓库的 id 是 `domain-historical-figures`。
fn dict_id(stem: &str) -> String {
    if stem == GENERAL_DICTIONARY {
        return stem.to_owned();
    }
    format!("domain-{}", stem.replace('_', "-"))
}

/// 已经装上了：附加词库按文件名，释义表按语言。
fn installed(library: &Library, id: &str) -> bool {
    if let Some(code) = id.strip_prefix("glossary-") {
        return code
            .parse::<Language>()
            .is_ok_and(|language| library.glossary_languages.contains(&language));
    }
    library
        .dictionaries
        .iter()
        .chain(&library.domains)
        .any(|entry| entry.stem == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Entry;

    fn entry(stem: &str) -> Entry {
        Entry {
            stem: stem.to_owned(),
            name: stem.to_owned(),
            entries: 1,
            license: String::new(),
            version: String::new(),
            broken: false,
        }
    }

    fn config(domains: &[&str], disabled: &[&str], learning: &str) -> Config {
        let mut config = Config::default();
        config.dictionaries.domains = domains.iter().map(|s| (*s).to_owned()).collect();
        config.dictionaries.disabled = disabled.iter().map(|s| (*s).to_owned()).collect();
        config.general.learning_language = learning.to_owned();
        config
    }

    #[test]
    fn opened_domains_general_and_missing_glossary_are_wanted() {
        let library = Library {
            domains: vec![entry("idioms")],
            glossary_languages: vec![Language::Chinese, Language::English],
            ..Library::default()
        };
        let config = config(&["idioms", "law", "historical_figures"], &[], "ja");
        assert_eq!(
            wanted(&config, &library),
            [
                "domain-law",
                "domain-historical-figures",
                "general",
                "glossary-ja"
            ]
        );
    }

    #[test]
    fn disabled_general_and_installed_ones_are_skipped() {
        let library = Library {
            domains: vec![entry("idioms")],
            dictionaries: vec![entry("domain-law")],
            glossary_languages: vec![Language::English, Language::German],
            ..Library::default()
        };
        let config = config(&["law", "food"], &["general"], "de");
        assert_eq!(wanted(&config, &library), ["domain-food"]);
    }
}
