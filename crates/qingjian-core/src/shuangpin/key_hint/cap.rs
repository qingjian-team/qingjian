//! 键盘上的一个键。

use super::KeyState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyCap {
    pub key: char,

    /// 占了这个键的翘舌声母（`zh` / `ch` / `sh`）；其余声母就是键名本身，不另标。
    pub initial: Option<String>,

    /// 这个键当韵母时的写法，`v` 已换成 `ü`。
    pub finals: Vec<String>,

    pub state: KeyState,
}
