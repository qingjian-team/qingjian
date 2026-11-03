//! [`SyncEvent`]：同步服务告诉界面的事。霓虹灯只看这些事件。

use super::received_item::ReceivedItem;

#[derive(Debug, Clone, PartialEq)]
pub enum SyncEvent {
    /// 附近的设备有变化，界面重读 [`crate::service::SyncService::nearby`]。
    NearbyChanged,

    /// 配对成功。
    Paired { name: String },

    /// 配对失败（码不对、过期、连不上）。
    PairFailed,

    /// 开始收对方的内容。没有要收的不发，所以陌生设备连进来不会亮灯。
    ReceiveStarted { peer: String },

    /// 收内容的进度。`bytes_per_second` 给霓虹灯选颜色，刚开始测不准时为 0。
    ReceiveProgress {
        peer: String,

        received: u64,

        total: u64,

        bytes_per_second: u64,
    },

    /// 收完了。`newest` 是这批里最新的一条没被删的，它比本机所有内容都新时才有，界面据此写系统剪贴板。
    ReceiveFinished {
        peer: String,

        newest: Option<ReceivedItem>,
    },

    /// 收到一半失败。
    ReceiveFailed { peer: String },

    /// 发不过去：对方在线却连不上或中途断开。对方本来就不在线不算，上线后会补传。
    SendFailed { peer: String },
}
