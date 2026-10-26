//! 用户的双拼方案：用户数据目录 `schemes/<id>/scheme.toml` 里写 `type = "shuangpin"` 的那些。
//!
//! 与形码方案共用一个 `schemes/` 目录、同一套 id 规则（见 [`crate::code_schemes::is_valid_id`]）；
//! 内置九套编在 Core 里（见 `qingjian_core::ShuangpinScheme`），这里的方案可以拿其中一套当底子只改几个键。
//! 写法见用户文档「双拼」一节。

mod descriptor;
mod entry;

use std::path::Path;
use std::sync::Arc;

use qingjian_core::{ShuangpinLayout, ShuangpinScheme};

use self::descriptor::Descriptor;
pub use self::entry::ShuangpinSchemeEntry;
use crate::code_schemes::{DESCRIPTOR, DIR, WUBI86, is_valid_id};

/// `scheme.toml` 里 `type` 写这个就是双拼方案。
pub const KIND: &str = "shuangpin";

/// 全部可用的双拼方案：随包的在前，用户的在后，各自按 id 排；同 id 时用户的占随包那个位置。
pub fn list(user_dir: Option<&Path>, bundled: Option<&Path>) -> Vec<ShuangpinSchemeEntry> {
    let mut entries = bundled.map_or_else(Vec::new, scan);
    if let Some(user_dir) = user_dir {
        for entry in scan(&user_dir.join(DIR)) {
            match entries.iter_mut().find(|bundled| bundled.id == entry.id) {
                Some(slot) => *slot = entry,
                None => entries.push(entry),
            }
        }
    }
    entries
}

/// 按 id 找一套方案；找不到为 `None`。
pub fn find(
    id: &str,
    user_dir: Option<&Path>,
    bundled: Option<&Path>,
) -> Option<ShuangpinSchemeEntry> {
    list(user_dir, bundled)
        .into_iter()
        .find(|entry| entry.id == id)
}

/// 读一个方案目录（目录名就是 id）；不是双拼方案或写错了为 `None`（写错了记日志）。
pub fn from_dir(dir: &Path) -> Option<ShuangpinSchemeEntry> {
    read(dir)
}

fn scan(dir: &Path) -> Vec<ShuangpinSchemeEntry> {
    let Ok(children) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut entries: Vec<ShuangpinSchemeEntry> = children
        .filter_map(Result::ok)
        .map(|child| child.path())
        .filter(|path| path.is_dir())
        .filter_map(|path| read(&path))
        .collect();
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    entries
}

/// 读一个方案目录；不是双拼方案时静默跳过，是但写错了记日志跳过（一套坏了不能拖垮别的）。
fn read(dir: &Path) -> Option<ShuangpinSchemeEntry> {
    let path = dir.join(DESCRIPTOR);
    let text = std::fs::read_to_string(&path).ok()?;
    let descriptor: Descriptor = match toml::from_str(&text) {
        Ok(descriptor) => descriptor,
        // 形码方案的 scheme.toml 也会走到这里，解析不了的交给形码那边报
        Err(_) => return None,
    };
    if descriptor.kind != KIND {
        return None;
    }
    let id = dir.file_name()?.to_str()?.to_owned();
    // `wubi86` 在 `scheme` 里是老配置「拼音关」的写法，不能当双拼方案的 id
    if !is_valid_id(&id) || id == WUBI86 {
        tracing::warn!(dir = %dir.display(), "双拼方案的目录名不能当 id，跳过");
        return None;
    }
    let label = match descriptor.name.trim() {
        "" => id.clone(),
        name => name.to_owned(),
    };
    let meta = descriptor.meta.clone();
    let layout = match layout(&id, descriptor) {
        Ok(layout) => layout,
        Err(error) => {
            tracing::warn!(file = %path.display(), %error, "双拼方案写得不对，跳过");
            return None;
        }
    };
    Some(ShuangpinSchemeEntry {
        id,
        label,
        meta,
        layout: Arc::new(layout),
    })
}

/// 底子加上改动，校验后返回键位；打不出来的音节只记日志。
fn layout(id: &str, descriptor: Descriptor) -> Result<ShuangpinLayout, String> {
    let mut layout = match descriptor.base.trim() {
        "" => ShuangpinLayout::new(id),
        base => base
            .parse::<ShuangpinScheme>()
            .map_err(|_| format!("base = {base:?} 不是内置双拼方案"))?
            .layout()
            .clone()
            .with_key(id),
    };
    for (key, initial) in descriptor.initials {
        let initial = initial.trim();
        layout.set_initial(single_key(&key)?, (!initial.is_empty()).then_some(initial));
    }
    for (key, finals) in descriptor.finals {
        layout.set_finals(single_key(&key)?, finals.into_vec());
    }
    for (syllable, spellings) in descriptor.zero {
        layout.set_zero_initial(syllable.trim(), spellings.into_vec());
    }
    let unreachable = layout.check()?;
    if !unreachable.is_empty() {
        tracing::warn!(id, ?unreachable, "双拼方案里这些音节打不出来");
    }
    Ok(layout)
}

fn single_key(key: &str) -> Result<char, String> {
    let mut chars = key.trim().chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Ok(c),
        _ => Err(format!("{key:?} 不是一个键")),
    }
}

#[cfg(test)]
mod tests;
