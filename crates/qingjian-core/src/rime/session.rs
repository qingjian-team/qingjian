//! 每个输入上下文持有独立 Rime 会话，候选选择交回原生 selector。
use std::ffi::{CStr, CString, c_char};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

use super::commit::Commit;
use super::context::Context;
use super::runtime::Runtime;
use super::snapshot::Snapshot;
use super::{RimeCandidate, RimeError, RimeMenu, RimeOptions};
use crate::{Candidate, CandidateKind};

pub(crate) struct Session {
    runtime: Arc<Runtime>,

    id: usize,

    token: u64,

    revision: u64,

    schema: String,
}

impl Session {
    pub fn open(options: RimeOptions) -> Result<Self, RimeError> {
        let schema = options.schema.clone();
        Self::create(Runtime::open(options)?, schema)
    }

    fn create(runtime: Arc<Runtime>, schema: String) -> Result<Self, RimeError> {
        let name = CString::new(schema.as_str()).map_err(|_| RimeError::Schema(schema.clone()))?;
        let guard = runtime.lock();
        let id = unsafe { (runtime.api.create)() };
        if id == 0 {
            return Err(RimeError::Session);
        }
        if unsafe { (runtime.api.select_schema)(id, name.as_ptr()) } == 0 {
            unsafe {
                (runtime.api.destroy)(id);
            }
            return Err(RimeError::Schema(schema));
        }
        drop(guard);
        Ok(Self {
            runtime,
            id,
            token: NEXT_SESSION.fetch_add(1, Ordering::Relaxed),
            revision: 0,
            schema,
        })
    }

    pub fn fork(&self) -> Result<Self, RimeError> {
        let schema = {
            let _guard = self.runtime.lock();
            let mut name = vec![0; 4096];
            if unsafe { (self.runtime.api.current_schema)(self.id, name.as_mut_ptr(), name.len()) }
                != 0
            {
                unsafe { copy_string(name.as_ptr()) }
            } else {
                self.schema.clone()
            }
        };
        Self::create(Arc::clone(&self.runtime), schema)
    }

    pub fn process(&mut self, key: i32, modifiers: i32) -> (bool, Option<String>) {
        let _guard = self.runtime.lock();
        self.revision = self.revision.wrapping_add(1);
        let consumed = unsafe { (self.runtime.api.process)(self.id, key, modifiers) } != 0;
        (consumed, self.read_commit())
    }

    pub fn select(&mut self, candidate: &RimeCandidate) -> Option<String> {
        if candidate.session != self.token || candidate.revision != self.revision {
            return None;
        }
        let _guard = self.runtime.lock();
        self.revision = self.revision.wrapping_add(1);
        unsafe {
            (self.runtime.api.select)(self.id, candidate.index);
        }
        self.read_commit()
    }

    pub fn delete(&mut self, candidate: &RimeCandidate) -> bool {
        if candidate.session != self.token || candidate.revision != self.revision {
            return false;
        }
        let _guard = self.runtime.lock();
        self.revision = self.revision.wrapping_add(1);
        unsafe { (self.runtime.api.delete)(self.id, candidate.index) != 0 }
    }

    pub fn clear(&mut self) {
        let snapshot = self.snapshot();
        let _guard = self.runtime.lock();
        self.revision = self.revision.wrapping_add(1);
        unsafe {
            // 原生方案菜单属于 switcher，clear_composition 只清普通输入上下文。
            if snapshot.input.is_empty() && !snapshot.candidates.items.is_empty() {
                (self.runtime.api.process)(self.id, 0xff1b, 0);
            }
            (self.runtime.api.clear)(self.id);
        }
        // 丢弃隐私边界或取消输入前尚未读取的上屏，不带到下一轮按键。
        self.read_commit();
    }

    pub fn commit(&mut self) -> String {
        let _guard = self.runtime.lock();
        self.revision = self.revision.wrapping_add(1);
        unsafe {
            (self.runtime.api.commit)(self.id);
        }
        self.read_commit().unwrap_or_default()
    }

    pub fn set_input(&mut self, text: &str) {
        if let Ok(text) = CString::new(text) {
            let _guard = self.runtime.lock();
            self.revision = self.revision.wrapping_add(1);
            unsafe {
                (self.runtime.api.set_input)(self.id, text.as_ptr());
            }
        }
    }

    pub fn option(&mut self, name: &CStr, value: bool) {
        let _guard = self.runtime.lock();
        self.revision = self.revision.wrapping_add(1);
        unsafe {
            (self.runtime.api.set_option)(self.id, name.as_ptr(), i32::from(value));
        }
    }

    pub fn ascii_mode(&self) -> bool {
        self.get_option(c"ascii_mode")
    }

    pub fn get_option(&self, name: &CStr) -> bool {
        let _guard = self.runtime.lock();
        unsafe { (self.runtime.api.get_option)(self.id, name.as_ptr()) != 0 }
    }

    pub fn page(&mut self, next: bool) {
        let _guard = self.runtime.lock();
        self.revision = self.revision.wrapping_add(1);
        unsafe {
            (self.runtime.api.change_page)(self.id, i32::from(!next));
        }
    }

    pub fn set_caret(&mut self, position: usize) {
        let _guard = self.runtime.lock();
        self.revision = self.revision.wrapping_add(1);
        unsafe {
            (self.runtime.api.set_caret)(self.id, position);
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        let _guard = self.runtime.lock();
        let mut result = Snapshot::default();
        unsafe {
            result.input = copy_string((self.runtime.api.input)(self.id));
            result.caret = (self.runtime.api.caret)(self.id).min(result.input.len());
            let mut context = Context {
                data_size: (std::mem::size_of::<Context>() - 4) as i32,
                ..Context::default()
            };
            if (self.runtime.api.context)(self.id, &mut context) == 0 {
                return result;
            }
            result.preedit = copy_string(context.composition.preedit);
            let mut cursor =
                (context.composition.cursor_pos.max(0) as usize).min(result.preedit.len());
            while !result.preedit.is_char_boundary(cursor) {
                cursor -= 1;
            }
            let page = context.menu.page_no.max(0) as usize;
            let page_size = context.menu.page_size.max(1) as usize;
            result.menu = RimeMenu {
                page,
                page_size,
                last_page: context.menu.is_last_page != 0,
                highlighted: context.menu.highlighted_candidate_index.max(0) as usize,
                cursor: result.preedit[..cursor].chars().count(),
            };
            let keys = copy_string(context.menu.select_keys);
            let count = context.menu.num_candidates.max(0) as usize;
            if !context.menu.candidates.is_null() {
                for index in 0..count {
                    let native = &*context.menu.candidates.add(index);
                    let text = copy_string(native.text);
                    let label = if !context.select_labels.is_null() {
                        copy_string(*context.select_labels.add(index))
                    } else {
                        keys.chars()
                            .nth(index)
                            .map_or_else(|| (index + 1).to_string(), |c| c.to_string())
                    };
                    let kind = if text.is_ascii() {
                        CandidateKind::English
                    } else {
                        CandidateKind::Chinese
                    };
                    result.candidates.items.push(Candidate {
                        text,
                        kind,
                        syllables: Vec::new(),
                        reading: None,
                        translation: None,
                        aux_code: None,
                        rime: Some(RimeCandidate {
                            session: self.token,
                            revision: self.revision,
                            index: page * page_size + index,
                            comment: copy_string(native.comment),
                            label,
                        }),
                    });
                }
            }
            (self.runtime.api.free_context)(&mut context);
        }
        result
    }

    // 调用时必须持有运行库锁；在 free_commit 前复制文本，返回值不借用原生内存。
    fn read_commit(&self) -> Option<String> {
        let mut commit = Commit {
            data_size: (std::mem::size_of::<Commit>() - 4) as i32,
            ..Commit::default()
        };
        unsafe {
            if (self.runtime.api.get_commit)(self.id, &mut commit) == 0 {
                return None;
            }
            let text = copy_string(commit.text);
            (self.runtime.api.free_commit)(&mut commit);
            (!text.is_empty()).then_some(text)
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _guard = self.runtime.lock();
        unsafe {
            (self.runtime.api.destroy)(self.id);
        }
    }
}

unsafe fn copy_string(pointer: *const c_char) -> String {
    if pointer.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(pointer) }
            .to_string_lossy()
            .into_owned()
    }
}
