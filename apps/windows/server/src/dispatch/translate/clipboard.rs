//! 只有备用译文的确认动作才写剪贴板；读取选区阶段不接触剪贴板。

pub(in crate::dispatch) fn copy_text(text: &str) -> Result<(), &'static str> {
    #[cfg(all(windows, not(test)))]
    {
        let mut clipboard =
            arboard::Clipboard::new().map_err(|_| "剪贴板暂不可用，请稍后按 Enter 重试")?;
        clipboard
            .set_text(text.to_owned())
            .map_err(|_| "复制失败，请稍后按 Enter 重试")
    }
    #[cfg(any(not(windows), test))]
    {
        let _ = text;
        Err("剪贴板不可用")
    }
}
