//! 一次按键：键与修饰键。

use super::KeyCode;
use crate::ui::Modifiers;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyPress {
    pub key: KeyCode,

    pub modifiers: Modifiers,
}

impl KeyPress {
    pub fn new(key: KeyCode, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }
}
