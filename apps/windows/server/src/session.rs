use qingjian_platform::protocol::SessionId;

/// 一个 TSF 文档对应的会话状态。
///
/// macOS 的 IMK 里 Engine 是进程级单例（`thread_local`），一次只有一个应用在组句；Windows 的 Server
/// 同时服务多个应用进程，所以每个会话要各自记住自己的组句进度。真正的组句状态（缓冲区、候选页、
/// 高亮）在接上 Engine 后填进来，现在先占位。
pub struct Session {
    /// 会话标识，由 DLL 分配。
    id: SessionId,
}

impl Session {
    pub fn new(id: SessionId) -> Self {
        Self { id }
    }

    pub fn id(&self) -> SessionId {
        self.id
    }
}
