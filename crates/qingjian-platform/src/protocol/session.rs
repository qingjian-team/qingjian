use serde::{Deserialize, Serialize};

/// 一次输入会话的标识。DLL 每进入一个 TSF 文档（`ITfContext`）就开一个会话，Server 按它分派状态。
///
/// 由 DLL 分配，进程内单调递增即可；Server 只做键，不解释其数值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SessionId(pub u64);
