use serde::{Deserialize, Serialize};

/// 一个日文译词及其假名读音。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JapaneseSense {
    /// 写法（汉字假名混写）。
    pub text: String,

    /// 全假名读音；模型给的不是纯假名时丢弃。
    pub reading: Option<String>,
}
