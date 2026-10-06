//! 原生后端通过 Engine 对外提供统一的输入、候选与上屏接口。
use super::Engine;
use super::input_log::InputSource;
use super::query::QuerySnapshot;
use crate::{Candidate, Query, RimeError, RimeOptions};

impl Engine {
    /// 初始化用户提供的 Rime/Lua 运行库，部署原始方案并创建独立会话。
    pub fn enable_rime(&mut self, options: RimeOptions) -> Result<(), RimeError> {
        let session = crate::rime::Session::open(options)?;
        self.clear();
        self.rime = Some(session);
        self.sync_rime();
        Ok(())
    }

    pub fn rime_enabled(&self) -> bool {
        self.rime.is_some()
    }

    /// 原始 X11 keysym / Rime 修饰键掩码。None 表示未启用该后端；
    /// 启用时返回是否吞键及已提交的文本，壳不再执行自己的拼音按键规则。
    pub fn process_rime_key(&mut self, key: i32, modifiers: i32) -> Option<(bool, Option<String>)> {
        if self.rime_enabled() && self.private {
            return Some((false, None));
        }
        if !self.rime_enabled() {
            return None;
        }
        if self.composition.is_empty() {
            self.composition_started = Some(std::time::Instant::now());
        }
        let result = self.rime.as_mut()?.process(key, modifiers);
        self.sync_rime();
        self.record_rime_commit(result.1.as_deref());
        Some(result)
    }

    /// 修改原生方案开关，如 ascii_punct、traditionalization 和 emoji。
    /// 返回 false 表示后端未启用或开关名称含 NUL。
    pub fn set_rime_option(&mut self, name: &str, value: bool) -> bool {
        let Ok(name) = std::ffi::CString::new(name) else {
            return false;
        };
        let Some(session) = &mut self.rime else {
            return false;
        };
        session.option(&name, value);
        self.sync_rime();
        true
    }

    pub fn rime_option(&self, name: &str) -> Option<bool> {
        let name = std::ffi::CString::new(name).ok()?;
        Some(self.rime.as_ref()?.get_option(&name))
    }

    pub fn rime_page(&mut self, next: bool) {
        if let Some(session) = &mut self.rime {
            session.page(next);
        }
        self.sync_rime();
    }

    pub(super) fn rime_set_caret(&mut self, position: usize) {
        if let Some(session) = &mut self.rime {
            session.set_caret(position);
        }
        self.sync_rime();
    }

    pub(super) fn rime_query(&self) -> Option<Query> {
        let snapshot = self.rime.as_ref()?.snapshot();
        *self.last_query.borrow_mut() = Some(QuerySnapshot {
            scope: snapshot.input.clone(),
            pinyin: snapshot.preedit.clone(),
            candidates: snapshot
                .candidates
                .items
                .iter()
                .take(QuerySnapshot::MAX_CANDIDATES)
                .map(|c| c.text.clone())
                .collect(),
            ..QuerySnapshot::default()
        });
        Some(Query {
            text: snapshot.input,
            cursor: snapshot.caret,
            typed_display: Some(snapshot.preedit),
            candidates: snapshot.candidates,
            rime_menu: Some(snapshot.menu),
            ..Query::default()
        })
    }

    pub(super) fn sync_rime(&mut self) {
        if let Some(session) = &self.rime {
            let snapshot = session.snapshot();
            self.english_mode = session.ascii_mode();
            self.composition.clear();
            let input = if snapshot.input.is_empty() && !snapshot.candidates.items.is_empty() {
                // 方案菜单不一定有 input，但仍是一轮需要画出来的组句。
                snapshot.preedit.as_str()
            } else {
                snapshot.input.as_str()
            };
            let caret = if snapshot.input.is_empty() {
                input.len()
            } else {
                snapshot.caret
            };
            self.composition.replace_native(input, caret);
        }
    }

    pub(super) fn rime_select(&mut self, candidate: &Candidate) -> Option<String> {
        let session = self.rime.as_mut()?;
        let commit = candidate
            .rime
            .as_ref()
            .and_then(|c| session.select(c))
            .unwrap_or_default();
        self.sync_rime();
        self.record_rime_commit(Some(&commit));
        Some(commit)
    }

    pub(super) fn record_rime_commit(&mut self, text: Option<&str>) {
        if let Some(text) = text.filter(|s| !s.is_empty()) {
            // Rime 自己负责词频；青简仍保留上文和显示过的译词记录。
            let keys = self
                .last_query
                .borrow()
                .as_ref()
                .map(|q| q.scope.clone())
                .unwrap_or_default();
            self.log_commit(&keys, text, InputSource::Rime);
            self.meter_commit(text, InputSource::Rime, false);
            self.punctuation.note_committed(text);
            self.history.record(text);
            self.remember_commit(super::LastCommit::plain(text));
        }
    }
}
