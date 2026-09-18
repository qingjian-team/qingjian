//! preedit 片段的画法。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreeditStyle {
    /// 敲的拼音：正常深浅。
    Typed,

    /// 光标后剩下的拼音：淡一点。
    Rest,

    /// 被纠错改掉的字母：淡且带删除线。
    Struck,

    /// 辅码码段：与剩余拼音同一个淡色，再压一道下划线区分。
    AuxCode,
}
