//! 窗口提交后回报实际完整可见的译词，索引必须属于当前帧。
use super::DisplayIdentity;
use qingjian_platform::protocol::SessionId;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayAcknowledged {
    /// 连接内会话编号，由 IPC 层重映射。
    pub session: SessionId,

    /// 当前展示身份。
    pub identity: DisplayIdentity,

    /// (候选槽位, Translation 义项编号)，不传候选文字。
    pub senses: Vec<(usize, usize)>,
}
