//! 键的状态，决定填充与文字颜色。

#[derive(Debug, Clone, Default, PartialEq)]
pub enum KeycapState {
    #[default]
    Normal,

    /// 刚敲下、等着配第二键。
    Pressed,

    /// 按下去能拼成这个音节。
    Next(String),

    /// 按下去拼不出音节。
    Dimmed,
}
