//! 剪贴板历史 → v 模式的 `cb` 参数。
//!
//! 数据来源是青简剪贴板自己那份历史（`%LOCALAPPDATA%\Qingjian\clipboard\history.db`），
//! 不去碰系统剪贴板：历史里已经有「最后一次复制」那条，而且用户能看见、能删。
//!
//! 只在参数里真的打了 `cb` 时才查（`shortcut::clipboard::substitute_with` 决定），
//! 一条 SQLite 查询、取一条文本，不做全表扫描。

use std::path::PathBuf;
use std::sync::Mutex;

use qingjian_clipboard::{ClipFilter, ClipStore};
use qingjian_core::shortcut::clipboard::Source;

/// 从剪贴板历史里取最近一条文本。
///
/// 打开失败（历史还没建、被别的进程锁着）时返回 `None`，`cb` 就退化成普通参数——
/// 这个功能不该因为剪贴板没装而让 v 模式失灵。
pub struct HistorySource {
    path: PathBuf,
    store: Mutex<Option<ClipStore>>,
}

impl HistorySource {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            store: Mutex::new(None),
        }
    }

    fn with_store<T>(&self, action: impl FnOnce(&ClipStore) -> T) -> Option<T> {
        let mut guard = self.store.lock().ok()?;
        if guard.is_none() {
            match ClipStore::open(&self.path, 1) {
                Ok(store) => *guard = Some(store),
                Err(error) => {
                    tracing::debug!(path = %self.path.display(), %error, "剪贴板历史打不开");
                    return None;
                }
            }
        }
        let store = guard.as_ref()?;
        Some(action(store))
    }
}

impl Source for HistorySource {
    fn text(&self) -> Option<String> {
        let text = self
            .with_store(|store| {
                store
                    .search("", ClipFilter::Text, 1)
                    .ok()
                    .and_then(|items| items.into_iter().next())
                    .map(|item| item.text)
            })
            .flatten()?;
        let trimmed = text.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    }
}

impl HistorySource {
    /// 青简剪贴板的历史库在 `%LOCALAPPDATA%\Qingjian\clipboard\history.db`——
    /// **不是**配置目录（`%APPDATA%`）那棵树下：那条路只有配置，剪贴板的数据在 Local。
    /// 取不到时给个读不到东西的空实现，`cb` 就退化成普通参数。
    pub fn system() -> Self {
        let path = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|base| base.join("Qingjian").join("clipboard").join("history.db"))
            .unwrap_or_default();
        Self::new(path)
    }
}
