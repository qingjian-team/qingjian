//! 辅码方案：`schemes/<id>/scheme.toml` 里写 `type = "aux"` 的那些，随包的与用户数据目录下的同一种格式。
//!
//! 与形码、双拼方案共用 `schemes/` 目录与 id 规则；与老位置 `codes/` 里导入的 `.qj` 一起叠加生效
//! （见 [`crate::code_tables::AuxCodeSources`]）。方案比单张码表多一样东西：词码规则，
//! 码表只收单字时按它给词拼出码。写法见用户文档「辅码」一节。

mod descriptor;
mod entry;

use std::path::Path;

use qingjian_dictionary::AuxWordRules;

use self::descriptor::Descriptor;
pub use self::entry::AuxSchemeEntry;
use crate::code_schemes::{DESCRIPTOR, DIR, is_valid_id};

/// `scheme.toml` 里 `type` 写这个就是辅码方案。
pub const KIND: &str = "aux";

/// 全部辅码方案：随包的在前，用户的在后，各自按 id 排；同 id 时用户的占随包那个位置。
pub fn list(user_dir: Option<&Path>, bundled: Option<&Path>) -> Vec<AuxSchemeEntry> {
    let mut entries = bundled.map_or_else(Vec::new, |dir| scan(dir, true));
    if let Some(user_dir) = user_dir {
        for entry in scan(&user_dir.join(DIR), false) {
            match entries.iter_mut().find(|bundled| bundled.id == entry.id) {
                Some(slot) => *slot = entry,
                None => entries.push(entry),
            }
        }
    }
    entries
}

/// 读一个用户方案目录（目录名就是 id）；不是辅码方案或写错了为 `None`（写错了记日志）。
pub fn from_dir(dir: &Path) -> Option<AuxSchemeEntry> {
    read(dir, false)
}

fn scan(dir: &Path, bundled: bool) -> Vec<AuxSchemeEntry> {
    let Ok(children) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut entries: Vec<AuxSchemeEntry> = children
        .filter_map(Result::ok)
        .map(|child| child.path())
        .filter(|path| path.is_dir())
        .filter_map(|path| read(&path, bundled))
        .collect();
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    entries
}

/// 读一个方案目录；不是辅码方案时静默跳过，是但写错了记日志跳过。
fn read(dir: &Path, bundled: bool) -> Option<AuxSchemeEntry> {
    let path = dir.join(DESCRIPTOR);
    let text = std::fs::read_to_string(&path).ok()?;
    let Ok(descriptor) = toml::from_str::<Descriptor>(&text) else {
        // 别的方案的 scheme.toml 也会走到这里，解析不了的交给形码那边报
        return None;
    };
    if descriptor.kind != KIND {
        return None;
    }
    let id = dir.file_name()?.to_str()?.to_owned();
    if !is_valid_id(&id) {
        tracing::warn!(dir = %dir.display(), "辅码方案的目录名不能当 id，跳过");
        return None;
    }
    let table = dir.join(&descriptor.table);
    if !table.is_file() {
        tracing::warn!(file = %table.display(), "辅码方案的码表不在，跳过");
        return None;
    }
    let words = match word_rules(&descriptor) {
        Ok(words) => words,
        Err(error) => {
            tracing::warn!(file = %path.display(), %error, "辅码方案的词码规则写得不对，跳过");
            return None;
        }
    };
    let label = match descriptor.name.trim() {
        "" => id.clone(),
        name => name.to_owned(),
    };
    Some(AuxSchemeEntry {
        id,
        label,
        meta: descriptor.meta,
        table,
        words,
        bundled,
    })
}

fn word_rules(descriptor: &Descriptor) -> Result<AuxWordRules, String> {
    let rules = descriptor
        .words
        .iter()
        .map(|(length, formula)| {
            length
                .trim()
                .parse::<usize>()
                .map(|length| (length, formula.as_str()))
                .map_err(|_| format!("{length:?} 不是词长"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    AuxWordRules::parse(rules).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests;
