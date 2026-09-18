//! 辅码态的 preedit 分段形态（触发键与码段）。

/// 辅码态在 preedit 里的两段：触发键按 `Typed` 画（不突出），码段是 [`MarkedKind::AuxCode`] 段。
///
/// 触发键只显示、不进缓冲区，也不参与候选；码段与拼音段分开记账（见 `Engine` 的 `aux_code` 字段）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuxSegment {
    /// 触发键（缺省 `;`）。用户敲的是它，但缓冲区里没有它。
    pub trigger: char,

    /// 码段：只 `a-z`，无光标概念、只从末尾增删。刚触发、还没敲码时是空串。
    pub code: String,
}

impl AuxSegment {
    pub fn new(trigger: char, code: String) -> Self {
        Self { trigger, code }
    }
}
