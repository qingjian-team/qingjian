use super::KeyResponse;

/// 一次 [`super::EngineClient::key`] 的应答。
pub enum KeyReply {
    /// 常规处理结果。
    Result(KeyResponse),

    /// 收到「翻译选中文字」快捷键：Server 请 DLL 读当前选区。
    NeedSelection {
        /// 请求标识，随 [`qingjian_platform::protocol::ClientMessage::Selection`] 带回。
        request: u64,
    },
}
