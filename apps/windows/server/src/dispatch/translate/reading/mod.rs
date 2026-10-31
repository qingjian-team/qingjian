//! 备用选区异步读取；COM 对象仅存在于工作线程，不进入 Router 或输入法宿主。

mod job;
#[cfg(all(windows, not(test)))]
mod native;

pub(super) use job::ReadJob;

pub(super) fn valid_text(text: &str) -> bool {
    !text.trim().is_empty() && !text.contains('\0') && text.encode_utf16().count() <= 500
}

#[cfg(test)]
mod tests {
    #[test]
    fn selection_limit_rejects_truncation_and_private_binary_content() {
        assert!(super::valid_text(&"中".repeat(500)));
        assert!(!super::valid_text(&"中".repeat(501)));
        assert!(super::valid_text(&"😀".repeat(250)));
        assert!(!super::valid_text(&"😀".repeat(251)));
        assert!(!super::valid_text(" \n\t"));
        assert!(!super::valid_text("hello\0world"));
    }
}
