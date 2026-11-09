//! 系统剪贴板。Linux 上剪贴板的内容由持有它的进程供应，所以整个进程只开一个、一直留着。

use std::sync::{Mutex, OnceLock};

fn clipboard() -> &'static Mutex<Option<arboard::Clipboard>> {
    static CLIPBOARD: OnceLock<Mutex<Option<arboard::Clipboard>>> = OnceLock::new();
    CLIPBOARD.get_or_init(|| {
        Mutex::new(
            arboard::Clipboard::new()
                .inspect_err(|error| tracing::warn!(%error, "打不开剪贴板"))
                .ok(),
        )
    })
}

/// 把文字放上剪贴板（输入框的复制、剪切）。
pub fn set_clipboard(text: &str) {
    if let Ok(mut guard) = clipboard().lock()
        && let Some(board) = guard.as_mut()
        && let Err(error) = board.set_text(text)
    {
        tracing::warn!(%error, "写剪贴板失败");
    }
}

/// 剪贴板上的文字；没有文字或读不了为 `None`。
pub(super) fn clipboard_text() -> Option<String> {
    let text = clipboard()
        .lock()
        .ok()?
        .as_mut()?
        .get_text()
        .inspect_err(|error| tracing::warn!(%error, "读剪贴板失败"))
        .ok()?;
    (!text.is_empty()).then_some(text)
}
