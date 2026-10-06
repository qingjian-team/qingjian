//! 雾凇全拼的数据与候选扩展，全部由青简 Rust 引擎执行。

mod calculator;
mod calendar;
mod data;
mod decoded;
mod filter;
mod money;
mod profile;
mod radical;
mod tools;

use std::path::Path;
use std::time::{Duration, Instant};

use crate::candidate::CandidateList;
use crate::engine::{Engine, Query, Timings};
use profile::Profile;

impl Engine {
    /// 读取用户提供的雾凇目录，中文词库缓存到独立目录；加载失败保留已有配置。
    pub fn load_rime_ice(&mut self, source: &Path, cache: &Path) -> std::io::Result<()> {
        let source = source.canonicalize()?;
        if self.ice.as_ref().is_some_and(|p| p.source == source) {
            return Ok(());
        }
        let profile = Profile::load(&source, cache)?;
        self.ice = Some(profile);
        self.clear();
        self.forget_span_cache();
        Ok(())
    }

    pub fn disable_rime_ice(&mut self) {
        if self.ice.is_none() {
            return;
        }
        self.ice = None;
        self.clear();
        self.forget_span_cache();
    }

    /// 雾凇扩展只作用于中文全拼，不改变其他输入方案。
    pub fn rime_ice_active(&self) -> bool {
        self.rime_ice_available() && !self.english_mode
    }

    /// 已加载的全拼能力不随会话中英模式改变，供壳在首个按键之前下发转发设置。
    pub fn rime_ice_available(&self) -> bool {
        self.ice.is_some() && self.shuangpin.is_none() && !self.zhuyin && self.code.is_none()
    }

    /// 特殊输入中的数字与标点仍是输入码，不作为选词或翻页键处理。
    pub fn rime_ice_input_char(&self, c: char) -> bool {
        self.rime_ice_active() && tools::accepts(&self.composition.typed_text(), c)
    }

    pub(super) fn query_ice(&self, start: Instant) -> Option<Query> {
        if let Some(query) = self.query_ice_radical(start) {
            return Some(query);
        }
        if !self.rime_ice_active() {
            return None;
        }
        let typed = self.composition.typed_scope();
        let mut items = self.ice.as_ref()?.special(&typed, &jiff::Zoned::now())?;
        // rq 等输入码也是简拼；快捷候选在前，普通词仍能翻页选择。
        if !typed.is_empty()
            && typed.bytes().all(|b| b.is_ascii_lowercase())
            && let Ok(normal) = self.query_phonetic(&typed, String::new(), start)
        {
            items.extend(normal.candidates.items);
        }
        let mut query = Query::custom_only(
            &self.composition.typed_text(),
            self.composition.cursor(),
            false,
            false,
            &typed,
            self.marked_rest(self.composition.rest()),
        );
        query.candidates = CandidateList { items };
        query.timings = Timings {
            parse: Duration::ZERO,
            lookup: Duration::ZERO,
            rank: start.elapsed(),
        };
        Some(query)
    }

    pub(super) fn has_ice_word(&self) -> bool {
        self.rime_ice_active()
            && self.ice.as_ref().is_some_and(|p| {
                let code = self.composition.typed_scope().to_ascii_lowercase();
                p.phrases.contains_key(&code) || p.mixed.contains_key(&code)
            })
    }
}

pub(super) use decoded::Decoded as IceDecoded;
pub(super) use profile::Profile as IceProfile;
