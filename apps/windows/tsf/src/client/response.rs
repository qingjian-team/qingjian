use qingjian_platform::protocol::{Frame, KeyOutcome};

/// Server 对一次按键的处理结果，DLL 据此决定是否吃键、往文档上屏什么、怎么画候选窗口。
pub struct KeyResponse {
    /// 这次按键吃掉（TSF 里返回「已处理」）还是放行给应用。
    pub outcome: KeyOutcome,

    /// 本次要立即上屏到文档的文本；没有则为 `None`。
    pub commit: Option<String>,

    /// 处理后要绘制的组句状态（preedit + 候选）；空帧表示收起候选窗口。
    pub frame: Frame,
}
