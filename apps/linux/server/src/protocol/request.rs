//! 带会话身份的 Linux 事件封套。
use super::LinuxEvent;
use qingjian_platform::protocol::SessionId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxRequest {
    pub session: SessionId,

    pub event: LinuxEvent,
}
