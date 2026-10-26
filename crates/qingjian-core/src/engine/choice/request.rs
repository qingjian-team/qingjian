//! 选词请求。

/// 一次选词请求：前文、敲的键的输入单元、要比的候选文本。三样都相同才算同一页，缓存按它对上。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChoiceRequest {
    pub context: String,

    /// 前文来自应用里光标前的文字（壳给的）；`false` 是应用给不出、退回本会话最近上屏的字。只写输入日志用。
    pub from_app: bool,

    pub typed: Vec<u32>,
    pub texts: Vec<String>,
}
