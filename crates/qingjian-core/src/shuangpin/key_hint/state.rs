//! 一个键在当前这一步的状态。

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyState {
    /// 还没敲声母：每个键照常显示。
    Idle,

    /// 刚敲下、等着配第二键的那个键。
    Pressed,

    /// 敲了声母后，按这个键能拼成的音节。
    Next(String),

    /// 敲了声母后，按这个键拼不出音节。
    Unavailable,
}
