//! 候选携带原生会话身份，避免陈旧候选或另一输入框的候选被选中。
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RimeCandidate {
    pub(crate) session: u64,

    pub(crate) revision: u64,

    pub(crate) index: usize,

    /// Lua / 反查 / 辅码等原始注释，与青简译文分别显示。
    pub comment: String,

    /// Rime 的选字键或自定义标签。
    pub label: String,
}
