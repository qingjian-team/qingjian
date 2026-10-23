//! 方向键这时管哪一区：拼音行还是候选。

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Focus {
    /// 左右键移拼音光标。
    Pinyin,

    /// 左右键在候选之间移动；新一轮输入从这里开始。
    #[default]
    Candidates,
}
