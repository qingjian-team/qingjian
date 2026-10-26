//! 一整块键盘。

use super::Keycap;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Keyboard {
    /// 从上到下的几排；第二、三排按真键盘往右错开。
    pub rows: Vec<Vec<Keycap>>,
}
