//! Fcitx 插件传给 Linux Server 的应用光标前后文。
use qingjian_core::SurroundingText;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSurrounding {
    pub before: String,

    pub after: String,
}

impl From<AppSurrounding> for SurroundingText {
    fn from(value: AppSurrounding) -> Self {
        Self {
            before: value.before,
            after: value.after,
        }
    }
}
