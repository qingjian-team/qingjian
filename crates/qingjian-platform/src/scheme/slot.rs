//! 壳手里「现在装着哪套方案」：按配置的两条轴装配引擎，形码方案与码表文件都没变时不重读码表。
//!
//! 每次配置热加载都会走到这里，而大多数时候改的是别的设置；8.9 万条的码表每次重读一遍是白花的。
//! 用户双拼方案只是一张小键位表，每次都重新读，变了才换进引擎。

use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;

use qingjian_core::Engine;

use super::PreparedScheme;
use crate::code_schemes::{self, CodeSchemeEntry};
use crate::shuangpin_schemes::{self, ShuangpinSchemeEntry};
use crate::{Config, DictionariesConfig, Scheme, scheme_label, scheme_name};

/// 码表与各本形码词库的修改时间与长度：都没变就当没改。
type Stamp = Vec<(Option<SystemTime>, u64)>;

/// 当前装上的用户双拼方案与形码方案（含码表文件的指纹）；没选或没装上为 `None`。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SchemeSlot {
    /// 已装入的用户双拼布局。
    shuangpin: Option<ShuangpinSchemeEntry>,

    /// 已装入的形码方案与码表指纹。
    code: Option<(CodeSchemeEntry, Stamp)>,
}

impl SchemeSlot {
    /// 按配置装配引擎：拼音侧（`[general] scheme`：全拼 / 双拼 / 用户双拼 / 注音 / 关）与
    /// 形码侧（`[general] code`，加上 `[code.<id>]` 里用户改的规则）。两边都开就是混输，见 `Engine::query_mixed`。
    ///
    /// 选了的方案找不到或读不了时警告并退回拼音（配置说五笔、引擎一个字都打不出更糟）；用户双拼找不到退回全拼。
    /// 每次都重新找方案（用户可能刚换了方案、刚放进新方案），但找到的形码方案与装着的是同一套、码表文件也没动时不重读，
    /// 只改规则。`bundled` 是随包的 `schemes` 目录。
    pub fn apply(
        &mut self,
        engine: &mut Engine,
        config: &Config,
        user_dir: Option<&Path>,
        bundled: Option<&Path>,
    ) {
        let prepared = self.prepare(config, user_dir, bundled, engine.code_scheme().is_some());
        self.apply_prepared(engine, prepared);
    }

    /// 读取方案与变化的码表，供后台线程使用；`code_loaded` 表示引擎仍持有当前码表。
    pub fn prepare(
        &self,
        config: &Config,
        user_dir: Option<&Path>,
        bundled: Option<&Path>,
        code_loaded: bool,
    ) -> PreparedScheme {
        let pinyin = config.general.scheme();
        let id = config.general.custom_shuangpin();
        let shuangpin = match (pinyin, id.as_deref()) {
            (Scheme::Custom, Some(id)) => {
                let found = shuangpin_schemes::find(id, user_dir, bundled);
                if found.is_none() {
                    tracing::warn!(
                        id,
                        "custom shuangpin scheme unavailable, falling back to pinyin"
                    );
                }
                found
            }
            _ => None,
        };
        let entry = config.general.code().and_then(|id| {
            let found = code_schemes::find(&id, user_dir, bundled);
            if found.is_none() {
                tracing::warn!(id, "code scheme unavailable, falling back to pinyin");
            }
            found.map(|mut entry| {
                entry
                    .words
                    .retain(|path| is_enabled(&config.dictionaries, path));
                entry.with_rules(config.code_rules(&id))
            })
        });
        let wanted = entry.map(|entry| {
            let stamp = stamp(&entry);
            (entry, stamp)
        });
        if let (Some((entry, stamp)), Some((loaded, loaded_stamp))) = (&wanted, &self.code)
            && same_table(entry, loaded)
            && stamp == loaded_stamp
            && code_loaded
        {
            return PreparedScheme {
                slot: Self {
                    shuangpin,
                    code: wanted,
                },
                pinyin,
                code: None,
            };
        }
        let mut code = None;
        let loaded = wanted.and_then(|(entry, stamp)| match entry.load() {
            Ok(scheme) => {
                tracing::info!(
                    id = %entry.id,
                    table = %entry.table.display(),
                    words = entry.words.len(),
                    entries = scheme.table.len(),
                    "code scheme loaded"
                );
                code = Some(scheme);
                Some((entry, stamp))
            }
            Err(error) => {
                tracing::error!(%error, table = %entry.table.display(), "failed to load code table, falling back to pinyin");
                None
            }
        });
        PreparedScheme {
            slot: Self {
                shuangpin,
                code: loaded,
            },
            pinyin,
            code,
        }
    }

    /// 应用已经读好的方案，不扫描目录或读取码表；与准备时的引擎、方案状态配对使用。
    pub fn apply_prepared(&mut self, engine: &mut Engine, prepared: PreparedScheme) {
        if prepared.slot.shuangpin.is_none() || prepared.slot.shuangpin != self.shuangpin {
            match &prepared.slot.shuangpin {
                Some(entry) => engine.set_shuangpin_layout(Some(Arc::clone(&entry.layout))),
                None => engine.set_shuangpin(prepared.pinyin.shuangpin()),
            }
        }
        engine.set_zhuyin_mode(prepared.pinyin == Scheme::Zhuyin);
        if let Some(scheme) = prepared.code {
            engine.set_code_scheme(Some(scheme));
        } else if let Some((entry, _)) = &prepared.slot.code {
            engine.set_code_rules(entry.auto_commit, entry.top);
            engine.set_mixed_code_rules(entry.mixed_auto_commit, entry.mixed_top);
        } else {
            engine.set_code_scheme(None);
        }
        engine.set_phonetic(prepared.pinyin.is_on() || prepared.slot.code.is_none());
        *self = prepared.slot;
    }

    /// 装上的用户双拼方案的名字。
    pub fn shuangpin_label(&self) -> Option<&str> {
        self.shuangpin.as_ref().map(|entry| entry.label.as_str())
    }

    /// 装上的形码方案的名字。
    pub fn code_label(&self) -> Option<&str> {
        self.code.as_ref().map(|(entry, _)| entry.label.as_str())
    }

    /// 状态条上的方案名，见 [`scheme_label`]。
    pub fn scheme_label(&self, pinyin: Scheme) -> String {
        scheme_label(pinyin, self.shuangpin_label(), self.code_label())
    }

    /// 主题显示的方案名，见 [`scheme_name`]。
    pub fn scheme_name(&self, pinyin: Scheme) -> String {
        scheme_name(pinyin, self.shuangpin_label(), self.code_label())
    }
}

/// 换了码表或组词规则才重装方案；只改按键规则不用。
fn same_table(a: &CodeSchemeEntry, b: &CodeSchemeEntry) -> bool {
    a.id == b.id
        && a.table == b.table
        && a.words == b.words
        && a.code_length == b.code_length
        && a.phrase_rule == b.phrase_rule
}

/// 形码词库与拼音词库共用 `[dictionaries] disabled`，按文件名开关。
fn is_enabled(config: &DictionariesConfig, path: &Path) -> bool {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .is_none_or(|stem| config.is_enabled(stem))
}

fn stamp(entry: &CodeSchemeEntry) -> Stamp {
    std::iter::once(&entry.table)
        .chain(&entry.words)
        .map(|path| {
            std::fs::metadata(path).map_or((None, 0), |metadata| {
                (metadata.modified().ok(), metadata.len())
            })
        })
        .collect()
}
