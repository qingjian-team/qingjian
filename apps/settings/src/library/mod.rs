//! 词库、码表与输入方案的清单：随包的、用户导入的各有哪些，名字、条数、许可证。进词库页（或导入、移除）时扫一次，不每帧读盘。
//! 移除是挪进目录里的 `removed/`，不真删。

mod entry;
mod kind;

pub use entry::Entry;
pub use kind::Kind;

use std::path::{Path, PathBuf};

use qingjian_dictionary::import::code_words_scheme;
use qingjian_dictionary::{AuxCodeTable, CodeTable, Dictionary, aux_code_table_info};
use qingjian_platform::aux_schemes::{self, AuxSchemeEntry};
use qingjian_platform::code_schemes::{self, CodeSchemeEntry};
use qingjian_platform::shuangpin_schemes::{self, ShuangpinSchemeEntry};

use crate::system::paths;

#[derive(Debug, Clone, Default)]
pub struct Library {
    pub domains: Vec<Entry>,

    pub dictionaries: Vec<Entry>,

    /// 导入的形码词库，名字后面带着给哪套方案。
    pub code_words: Vec<Entry>,

    pub bundled_codes: Vec<Entry>,

    pub codes: Vec<Entry>,

    /// `schemes/` 里的辅码方案（随包的与用户的），开关与 `codes/` 的码表同一项；不在设置里移除。
    pub aux_schemes: Vec<Entry>,

    /// 能选的形码方案（内置五笔与用户目录 `schemes/` 下的），只读描述、不读码表。
    pub code_schemes: Vec<CodeSchemeEntry>,

    /// 用户目录 `schemes/` 下的双拼方案（内置九套不在这里）。
    pub shuangpin_schemes: Vec<ShuangpinSchemeEntry>,
}

impl Library {
    pub fn scan() -> Self {
        let scan = |kind: Kind, dir: Option<PathBuf>| -> Vec<Entry> {
            dir.map(|dir| {
                kind.list(&dir)
                    .into_iter()
                    .map(|(stem, path)| entry(kind, stem, &path))
                    .collect()
            })
            .unwrap_or_default()
        };
        let code_schemes = scan_code_schemes();
        Self {
            domains: scan(Kind::Dictionary, Kind::Dictionary.bundled_dir()),
            dictionaries: scan(Kind::Dictionary, Kind::Dictionary.user_dir()),
            code_words: scan_code_words(&code_schemes),
            bundled_codes: scan(Kind::CodeTable, Kind::CodeTable.bundled_dir()),
            codes: scan(Kind::CodeTable, Kind::CodeTable.user_dir()),
            aux_schemes: scan_aux_schemes(),
            code_schemes,
            shuangpin_schemes: scan_shuangpin_schemes(),
        }
    }

    /// 形码方案的名字；没装着这套方案时用 id。
    pub fn code_scheme_label(&self, id: &str) -> String {
        self.code_schemes
            .iter()
            .find(|entry| entry.id == id)
            .map_or_else(|| id.to_owned(), |entry| entry.label.clone())
    }

    /// 能选的辅码，与输入法挑的先后一致（见 `AuxCodeSources::options`）：随包的、导入的、方案；id 重了的只留前面那套。
    pub fn aux_options(&self) -> Vec<&Entry> {
        let mut options: Vec<&Entry> = Vec::new();
        for entry in self
            .bundled_codes
            .iter()
            .chain(&self.codes)
            .chain(&self.aux_schemes)
        {
            if !options.iter().any(|seen| seen.stem == entry.stem) {
                options.push(entry);
            }
        }
        options
    }

    /// 只重扫方案，切回窗口与进「输入」页时用。形码与双拼只读 `scheme.toml`；辅码方案要数条数，读一遍码表。
    pub fn rescan_schemes(&mut self) {
        self.aux_schemes = scan_aux_schemes();
        self.code_schemes = scan_code_schemes();
        self.shuangpin_schemes = scan_shuangpin_schemes();
    }
}

fn scan_aux_schemes() -> Vec<Entry> {
    aux_schemes::list(paths::user_dir().as_deref(), bundled_schemes().as_deref())
        .into_iter()
        .map(aux_scheme_entry)
        .collect()
}

fn aux_scheme_entry(scheme: AuxSchemeEntry) -> Entry {
    let table = AuxCodeTable::from_path(&scheme.table);
    if let Err(error) = &table {
        tracing::warn!(path = %scheme.table.display(), %error, "辅码方案的码表打不开");
    }
    Entry {
        stem: scheme.id,
        name: scheme.label,
        entries: table.as_ref().map_or(0, AuxCodeTable::len),
        license: scheme.meta.license,
        broken: table.is_err(),
    }
}

fn scan_code_words(schemes: &[CodeSchemeEntry]) -> Vec<Entry> {
    let Some(dir) = Kind::CodeWords.user_dir() else {
        return Vec::new();
    };
    code_schemes::list_words(&dir)
        .into_iter()
        .map(|words| {
            let label = schemes
                .iter()
                .find(|entry| entry.id == words.scheme)
                .map_or(words.scheme.as_str(), |entry| entry.label.as_str());
            code_words_entry(words.stem, &words.path, label)
        })
        .collect()
}

/// 名字是「文件名（方案名）」：同一个导入入口进来的，得看得出哪本是五笔的。
fn code_words_entry(stem: String, path: &Path, scheme: &str) -> Entry {
    let table = CodeTable::from_path(path);
    if let Err(error) = &table {
        tracing::warn!(path = %path.display(), %error, "形码词库打不开");
    }
    Entry {
        name: format!("{stem}（{scheme}）"),
        stem,
        entries: table.as_ref().map_or(0, CodeTable::len),
        license: String::new(),
        broken: table.is_err(),
    }
}

fn scan_code_schemes() -> Vec<CodeSchemeEntry> {
    code_schemes::list(paths::user_dir().as_deref(), bundled_schemes().as_deref())
}

fn scan_shuangpin_schemes() -> Vec<ShuangpinSchemeEntry> {
    shuangpin_schemes::list(paths::user_dir().as_deref(), bundled_schemes().as_deref())
}

fn entry(kind: Kind, stem: String, path: &Path) -> Entry {
    match kind {
        Kind::Dictionary => match Dictionary::from_path(path) {
            Ok(dictionary) => {
                let metadata = dictionary.metadata();
                Entry {
                    name: metadata.map_or_else(|| stem.clone(), |m| m.name.clone()),
                    license: metadata.map_or_else(String::new, |m| m.license.clone()),
                    entries: dictionary.len(),
                    broken: false,
                    stem,
                }
            }
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "词库打不开");
                Entry {
                    name: stem.clone(),
                    stem,
                    entries: 0,
                    license: String::new(),
                    broken: true,
                }
            }
        },
        Kind::CodeWords => {
            let scheme = code_words_scheme(path).unwrap_or_default();
            code_words_entry(stem, path, &scheme)
        }
        Kind::CodeTable => {
            let info = aux_code_table_info(path);
            Entry {
                stem,
                name: info.name,
                entries: info.entries,
                license: info.license,
                broken: info.broken,
            }
        }
    }
}

/// 随包的 `dicts` / `codes` 目录，与各平台输入法读的同一处：
/// macOS 在 `Qingjian.app/Contents/Resources/`（设置程序将来也装在包里，先找自己所在的包，再找装好的输入法），
/// 其他平台在随包根的 `data/generated/`；都找不到时用开发布局（仓库的 `data/generated/`）。
fn bundled_dir(name: &str) -> Option<PathBuf> {
    paths::mac_resources()
        .into_iter()
        .map(|dir| dir.join(name))
        .find(|dir| dir.is_dir())
        .or_else(|| paths::bundled(&format!("data/generated/{name}")))
}

/// 随包的形码方案目录，与各平台输入法读的同一处：macOS 在 `Resources/schemes/`，其他平台在随包根的 `assets/schemes/`。
fn bundled_schemes() -> Option<PathBuf> {
    paths::mac_resources()
        .into_iter()
        .map(|dir| dir.join(code_schemes::DIR))
        .find(|dir| dir.is_dir())
        .or_else(|| paths::bundled(&format!("assets/{}", code_schemes::DIR)))
}

/// 导入无读音词表时的参考词库：随包基础词库加随包领域词库，读不到的跳过（导入报告里会说到）。
pub fn reference_dictionaries() -> Vec<Dictionary> {
    let mut references = Vec::new();
    let mut read = |path: &Path| match Dictionary::from_path(path) {
        Ok(dictionary) => references.push(dictionary),
        Err(error) => tracing::warn!(path = %path.display(), %error, "参考词库打不开"),
    };
    if let Some(base) = base_dictionary() {
        read(&base);
    }
    if let Some(dir) = Kind::Dictionary.bundled_dir() {
        for (_, path) in Kind::Dictionary.list(&dir) {
            read(&path);
        }
    }
    references
}

/// 随包基础词库：macOS 在 `Resources/dict.qj`，其他平台在随包根的 `data/generated/dict.qj`。
fn base_dictionary() -> Option<PathBuf> {
    paths::mac_resources()
        .into_iter()
        .map(|dir| dir.join("dict.qj"))
        .find(|path| path.is_file())
        .or_else(|| paths::bundled("data/generated/dict.qj"))
}

/// 挪进 `removed/`；输入法看到文件没了会自己卸掉。
pub fn remove(kind: Kind, stem: &str) -> Result<(), String> {
    let dir = kind.user_dir().ok_or("找不到数据目录")?;
    let (_, path) = kind
        .list(&dir)
        .into_iter()
        .find(|(name, _)| name == stem)
        .ok_or("文件已经不在了")?;
    let removed = dir.join("removed");
    std::fs::create_dir_all(&removed).map_err(|error| format!("移除失败：{error}"))?;
    let name = path.file_name().ok_or("文件名不对")?;
    std::fs::rename(&path, removed.join(name)).map_err(|error| format!("移除失败：{error}"))
}
