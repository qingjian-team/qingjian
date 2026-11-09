//! 连接代次、上下文身份与服务端帧版本共同标识一次展示。
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayIdentity {
    /// 插件当前连接代次。
    pub generation: u64,

    /// Fcitx InputContext UUID。
    pub context: String,

    /// Server 当前帧版本。
    pub revision: u64,
}
