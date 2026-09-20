//! 一条连接内的会话映射与独立协商状态。
use crate::protocol::DisplayIdentity;
use qingjian_platform::protocol::SessionId;

pub(super) struct Session {
    pub(super) global: SessionId,

    pub(super) identity: Option<DisplayIdentity>,

    pub(super) capabilities: bool,
}
