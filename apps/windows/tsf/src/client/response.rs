use qingjian_platform::protocol::{Frame, KeyOutcome};

/// Server 对一次按键的处理结果。
pub struct KeyResponse {
    /// 吃掉还是放行给应用。
    pub outcome: KeyOutcome,

    /// 本次要立即上屏到文档的文本。
    pub commit: Option<String>,

    /// 处理后的组句状态（preedit + 候选）；空帧表示收起候选窗口。
    pub frame: Frame,
}
