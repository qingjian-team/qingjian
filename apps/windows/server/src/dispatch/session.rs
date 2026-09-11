/// 一个活跃会话在 Server 侧记下的信息，开会话时由 DLL 报来。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SessionInfo {
    /// 宿主应用的 exe 文件名（`Code.exe`），查 `[apps]` 按应用设置用；DLL 取不到时为 `None`。
    pub(super) app: Option<String>,
}
