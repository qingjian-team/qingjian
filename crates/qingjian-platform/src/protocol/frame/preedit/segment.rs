use serde::{Deserialize, Serialize};

use qingjian_core::MarkedSegment;

use super::kind::PreeditKind;

/// preedit（组句拼音行）的一段。整段 preedit 是若干段按顺序拼起来，光标位置按拼接后的字符数算。
/// 是 Core 的 [`MarkedSegment`] 的可序列化镜像。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreeditSegment {
    /// 文本。
    pub text: String,

    /// 种类，决定样式。
    pub kind: PreeditKind,
}

impl From<&MarkedSegment> for PreeditSegment {
    fn from(segment: &MarkedSegment) -> Self {
        Self {
            text: segment.text.clone(),
            kind: segment.kind.into(),
        }
    }
}
