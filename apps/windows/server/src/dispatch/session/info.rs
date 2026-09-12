//! 会话信息。

/// 一个活跃会话在 Server 侧记下的信息，开会话时由 DLL 报来。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionInfo {
    /// 宿主应用的 exe 文件名（如 `Code.exe`），查 `[apps]` 用；取不到为 `None`。
    pub(crate) app: Option<String>,

    /// 该会话当前落在私密输入框里（DLL 随 `ClientMessage::Privacy` 报来）；焦点切回来时按它重设 Engine。
    pub(crate) private: bool,
}
