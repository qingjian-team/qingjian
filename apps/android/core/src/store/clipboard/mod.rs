//! 剪贴板面板的读写：壳把系统剪贴板里的文字、图片交进来，列表、删、清空都走 `qingjian_learning::ClipboardHistory`。
//! 不要句柄（不碰引擎），每次按数据目录打开文件——千条级几百 KB，读一遍是毫秒级。

mod item;
mod request;

pub use item::ClipboardItem;
pub use request::ClipboardRequest;

use std::path::Path;

use qingjian_learning::ClipboardHistory;

/// 跑一个请求，返回操作之后的整份列表（新的在前）。`limit` 是设置里的最大条数，每次都按它裁。
pub fn clipboard(user_dir: &Path, request: ClipboardRequest, limit: usize) -> Vec<ClipboardItem> {
    let mut history = ClipboardHistory::open(user_dir);
    let result = history.set_limit(limit).and_then(|()| match request {
        ClipboardRequest::List => Ok(()),
        ClipboardRequest::Add { text } => history.add(&text).map(drop),
        ClipboardRequest::Remove { index } => history.remove(index),
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
    let mut history = ClipboardHistory::open(user_dir);
    let result = history
        .set_limit(limit)
        .and_then(|()| history.add_image(bytes, extension).map(drop));
    if let Err(error) = result {
        tracing::warn!(%error, "剪贴板图片存不进去");
    }
    items(&history)
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
    fn requests_parse_from_shell_json() {
        let request: ClipboardRequest =
            serde_json::from_str(r#"{"op":"remove","index":3}"#).unwrap();
        assert!(matches!(request, ClipboardRequest::Remove { index: 3 }));
    }
}
