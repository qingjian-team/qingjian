/// 解码出的一个单元：双拼的一两个键或一个全拼音节。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    /// 敲的键（混输时全拼音节最多 6 个字符；用户自己敲的 `'` 单独一个单元）。
    pub keys: String,

    /// 翻出来的全拼：两键是完整音节，落单的一键是声母（`v` → `zh`）或元音（`a`），`'` 为空。
    pub pinyin: String,

    /// 是否是完整音节。
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
