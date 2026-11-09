//! [`PeerState`]：已配对设备连同它的状态。

use super::PeerStatus;
use crate::peers::Peer;

/// 一台已配对设备和它现在的状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerState {
    pub peer: Peer,

    pub status: PeerStatus,
}
