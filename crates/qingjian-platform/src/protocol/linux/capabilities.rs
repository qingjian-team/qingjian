//! 框架能力事实；输入策略由 Server 决定。
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    pub sensitive: bool,

    pub password: bool,

    pub disabled: bool,
}
