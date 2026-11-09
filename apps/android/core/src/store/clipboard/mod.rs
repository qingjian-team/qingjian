//! 剪贴板面板的读写：壳把系统剪贴板里的文字、图片交进来，列表、删、清空都走 `qingjian_learning::ClipboardHistory`。
//! 不要句柄（不碰引擎），每次按数据目录打开文件——千条级几百 KB，读一遍是毫秒级。

mod item;
mod replica;
mod request;

pub use item::ClipboardItem;
pub use replica::HistoryReplica;
pub use request::ClipboardRequest;

use std::path::Path;
use std::sync::{Mutex, PoisonError};

use qingjian_learning::ClipboardHistory;

/// 面板、监听和同步都读改写同一个历史文件，一次一个，不然后写的盖掉先写的。
pub(crate) static LOCK: Mutex<()> = Mutex::new(());

/// 跑一个请求，返回操作之后的整份列表（新的在前）。`limit` 是设置里的最大条数，每次都按它裁。
pub fn clipboard(user_dir: &Path, request: ClipboardRequest, limit: usize) -> Vec<ClipboardItem> {
    let _guard = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    let request = match request {
        ClipboardRequest::Edit { index, from, text } => match locate(user_dir, index, &from) {
            Some(index) if text != from && !text.trim().is_empty() => {
                ClipboardRequest::Edit { index, from, text }
            }
            _ => ClipboardRequest::List,
        },
        request => request,
    };
    // 改文字对别的设备是「旧的删掉、新的一条」，同桌面剪贴板
    if let ClipboardRequest::Remove { index } | ClipboardRequest::Edit { index, .. } = request {
        replica::mark_deleted(user_dir, index);
    }
    let mut history = ClipboardHistory::open(user_dir);
    let result = history.set_limit(limit).and_then(|()| match request {
        ClipboardRequest::List => Ok(()),
        ClipboardRequest::Add { text } => history.add(&text).map(drop),
        ClipboardRequest::Remove { index } => history.remove(index),
        ClipboardRequest::Edit { index, text, .. } => history
            .remove(index)
            .and_then(|()| history.add(&text).map(drop)),
        ClipboardRequest::Clear => history.clear(),
    });
    if let Err(error) = result {
        tracing::warn!(%error, "剪贴板历史写不进去");
    }
    items(&history)
}

/// 记一张复制的图片（`extension` 如 `png`），返回操作之后的列表。
pub fn clipboard_add_image(
    user_dir: &Path,
    bytes: &[u8],
    extension: &str,
    limit: usize,
) -> Vec<ClipboardItem> {
    let _guard = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    let mut history = ClipboardHistory::open(user_dir);
    let result = history
        .set_limit(limit)
        .and_then(|()| history.add_image(bytes, extension).map(drop));
    if let Err(error) = result {
        tracing::warn!(%error, "剪贴板图片存不进去");
    }
    items(&history)
}

/// 原文是 `from` 的文字条目现在排第几：先看 `index` 还是不是它，不是再从头找；已经没了为 `None`。
fn locate(user_dir: &Path, index: usize, from: &str) -> Option<usize> {
    let history = ClipboardHistory::open(user_dir);
    let entries = history.entries();
    let matches = |i: usize| {
        entries
            .get(i)
            .is_some_and(|e| !e.is_image() && e.text == from)
    };
    if matches(index) {
        Some(index)
    } else {
        (0..entries.len()).find(|&i| matches(i))
    }
}

fn items(history: &ClipboardHistory) -> Vec<ClipboardItem> {
    history
        .entries()
        .iter()
        .map(|entry| ClipboardItem {
            text: entry.text.clone(),
            image: history
                .image_path(entry)
                .map(|path| path.to_string_lossy().into_owned()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_list_remove() {
        let dir = std::env::temp_dir().join(format!("qj-android-clip-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        clipboard(&dir, ClipboardRequest::Add { text: "a".into() }, 1000);
        let items = clipboard(&dir, ClipboardRequest::Add { text: "b".into() }, 1000);
        assert_eq!(items[0].text, "b");
        let items = clipboard(&dir, ClipboardRequest::Remove { index: 0 }, 1000);
        assert_eq!(items.len(), 1);
        let items = clipboard_add_image(&dir, b"img", "png", 1);
        assert_eq!(items.len(), 1);
        assert!(
            items[0]
                .image
                .as_deref()
                .is_some_and(|p| p.ends_with(".png"))
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn edit_replaces_the_text_and_moves_it_to_the_front() {
        let dir = std::env::temp_dir().join(format!("qj-android-clip-edit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        clipboard(&dir, ClipboardRequest::Add { text: "a".into() }, 1000);
        clipboard(&dir, ClipboardRequest::Add { text: "b".into() }, 1000);
        let edit = |index, from: &str, text: &str| ClipboardRequest::Edit {
            index,
            from: from.into(),
            text: text.into(),
        };
        let items = clipboard(&dir, edit(1, "a", "a2"), 1000);
        let texts: Vec<_> = items.iter().map(|item| item.text.as_str()).collect();
        assert_eq!(texts, ["a2", "b"]);
        // 编辑期间进来一条新的，原来那条被挤到后面：按原文找到它
        clipboard(&dir, ClipboardRequest::Add { text: "c".into() }, 1000);
        let items = clipboard(&dir, edit(1, "b", "b2"), 1000);
        let texts: Vec<_> = items.iter().map(|item| item.text.as_str()).collect();
        assert_eq!(texts, ["b2", "c", "a2"]);
        // 空白不改
        let items = clipboard(&dir, edit(0, "b2", "  "), 1000);
        assert_eq!(items[0].text, "b2");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn requests_parse_from_shell_json() {
        let request: ClipboardRequest =
            serde_json::from_str(r#"{"op":"remove","index":3}"#).unwrap();
        assert!(matches!(request, ClipboardRequest::Remove { index: 3 }));
    }
}
