//! 尚未上屏的原样文本及其 UTF-8 字节光标。

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawPreedit {
    /// 与相同状态下 `Engine::take_raw()` 返回的完整文本一致。
    pub text: String,

    /// `text` 中的合法 UTF-8 字符边界，空文本为 0。
    pub cursor_bytes: usize,
}
