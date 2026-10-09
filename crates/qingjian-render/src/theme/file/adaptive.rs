//! 随外观变的值：写一个值两种外观共用，或写 `{ "light": …, "dark": … }` 各给一个。

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum Adaptive<T> {
    /// 浅色与深色各一个。
    Pair { light: T, dark: T },

    /// 两种外观共用。
    Same(T),
}

impl<T: Copy> Adaptive<T> {
    pub(crate) fn get(&self, dark: bool) -> T {
        match self {
            Self::Pair { light, dark: value } => {
                if dark {
                    *value
                } else {
                    *light
                }
            }
            Self::Same(value) => *value,
        }
    }
}
