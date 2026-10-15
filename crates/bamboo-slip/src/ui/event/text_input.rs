//! 文字输入：直接打的字、输入法上屏的字、粘贴的字都是 `Insert`；输入法正在组的字是 `Preedit`。
//!
//! 键盘上产生字符的键先作为 [`super::Event::Key`] 送一遍（给快捷键用），再作为 `Insert` 送一遍；
//! 收文字的元素不处理字符键，只处理 `Insert`，两边不会重复。

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextInput {
    Insert(String),

    /// 输入法正在组的字（还没上屏）；空串表示组字结束或取消。
    Preedit(String),
}
