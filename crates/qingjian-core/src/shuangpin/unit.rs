/// 解码出的一个单元：一两个键对应的全拼。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    /// 敲的键（1 或 2 个字符；用户自己敲的 `'` 单独一个单元）。
    pub keys: String,

    /// 翻出来的全拼：两键是完整音节，落单的一键是声母（`v` → `zh`）或元音（`a`），`'` 为空。
    pub pinyin: String,

    /// 是否是完整音节（两键）。
    pub complete: bool,
}

impl Unit {
    pub fn separator() -> Self {
        Self {
            keys: "'".to_owned(),
            pinyin: String::new(),
            complete: false,
        }
    }

    pub fn is_separator(&self) -> bool {
        self.pinyin.is_empty()
    }
}
