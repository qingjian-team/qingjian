//! 压在文字上的一道线：删除线穿过小写字母中部，下划线在基线下面一点；粗细都随字号。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decoration {
    Strikethrough,

    Underline,
}

impl Decoration {
    /// 线的顶边相对基线的偏移（向下为正，字号的倍数）。
    pub(super) fn offset(self) -> f32 {
        match self {
            Self::Strikethrough => -0.3,
            Self::Underline => 0.14,
        }
    }

    /// 线的粗细（像素），至少一像素。
    pub(super) fn thickness(size: f32) -> f32 {
        (size / 14.0).max(1.0)
    }
}
