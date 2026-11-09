//! 取清单的后台任务：在自己的线程里取，取到了写缓存，取完叫醒窗口。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use bamboo_slip::Waker;

use super::index_file::IndexFile;
use super::{DictEntry, IndexState, Status};

use crate::jobs::download::{Repo, fetch_small};

/// 缓存文件名，放在数据目录里（与 `update.json` 同一处）。
const CACHE_FILE: &str = "dict-index.json";

pub struct DictIndex {
    state: Arc<Mutex<IndexState>>,

    /// 后台线程取完时置上，`take_finished` 收一次。
    finished: Arc<AtomicBool>,
}

impl Default for DictIndex {
    /// 先读上次缓存的那份，取到新的之前就能列出来。
    fn default() -> Self {
        let mut state = IndexState::default();
        if let Some(path) = cache_path()
            && let Ok(bytes) = std::fs::read(&path)
            && let Ok(dicts) = IndexFile::parse(&bytes)
        {
            state.dicts = dicts;
            state.fetched_at = std::fs::metadata(&path)
                .and_then(|meta| meta.modified())
                .ok();
        }
        Self {
            state: Arc::new(Mutex::new(state)),
            finished: Arc::default(),
        }
    }
}

impl DictIndex {
    pub fn state(&self) -> IndexState {
        self.state
            .lock()
            .map(|state| state.clone())
            .unwrap_or_default()
    }

    /// 手上清单里的一本。
    pub fn entry(&self, id: &str) -> Option<DictEntry> {
        self.state().dicts.into_iter().find(|entry| entry.id == id)
    }

    pub fn start(&mut self, waker: Option<Waker>) {
        {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            if state.status == Status::Loading {
                return;
            }
            state.status = Status::Loading;
        }
        let state = Arc::clone(&self.state);
        let finished = Arc::clone(&self.finished);
        std::thread::spawn(move || {
            let urls = Repo::Dicts
                .raw_hosts()
                .into_iter()
                .map(|host| format!("{host}/index.json"))
                .collect();
            let result = fetch_small(urls, |bytes| {
                IndexFile::parse(bytes).map(|dicts| (dicts, bytes.to_vec()))
            });
            if let Ok(mut slot) = state.lock() {
                match result {
                    Ok((dicts, bytes)) => {
                        tracing::info!(count = dicts.len(), "dictionary index fetched");
                        save_cache(&bytes);
                        slot.dicts = dicts;
                        slot.fetched_at = Some(SystemTime::now());
                        slot.status = Status::Fetched;
                    }
                    Err(error) => {
                        tracing::warn!(error, "dictionary index not fetched");
                        slot.status = Status::Failed(error);
                    }
                }
            }
            finished.store(true, Ordering::Relaxed);
            if let Some(waker) = waker {
                waker.wake();
            }
        });
    }

    /// 后台取完了没有（取完后只返回一次 `true`），窗口据此重画。
    pub fn take_finished(&self) -> bool {
        self.finished.swap(false, Ordering::Relaxed)
    }
}

fn cache_path() -> Option<PathBuf> {
    crate::system::paths::user_dirs().map(|dirs| dirs.cache().join(CACHE_FILE))
}

/// 写临时文件再改名，半截的缓存不会留下。
fn save_cache(bytes: &[u8]) {
    let Some(path) = cache_path() else {
        return;
    };
    let part = path.with_extension("json.part");
    let saved = std::fs::write(&part, bytes).and_then(|()| std::fs::rename(&part, &path));
    if let Err(error) = saved {
        tracing::warn!(path = %path.display(), %error, "dictionary index cache not saved");
        let _ = std::fs::remove_file(&part);
    }
}
