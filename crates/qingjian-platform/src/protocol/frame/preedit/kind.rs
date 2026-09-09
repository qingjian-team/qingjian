use serde::{Deserialize, Serialize};

use qingjian_core::MarkedKind;

/// preedit 片段的种类，DLL 按它选样式。是 Core 的 [`MarkedKind`] 的可序列化镜像
/// （协议不直接用 Core 的内部枚举，免得两者耦合）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreeditKind {
    /// 用户敲的、参与本次候选的拼音（已按音节用 `'` 切开）。
    Typed,

    /// 光标停在中间时，作用域之后剩下的拼音：只显示不参与候选，画淡一点。
    Rest,

    /// 拼写纠错里被改掉的原字母：画删除线。
    Corrected,
}

impl From<MarkedKind> for PreeditKind {
    fn from(kind: MarkedKind) -> Self {
        match kind {
            MarkedKind::Typed => Self::Typed,
            MarkedKind::Rest => Self::Rest,
            MarkedKind::Corrected => Self::Corrected,
        }
    }
}
