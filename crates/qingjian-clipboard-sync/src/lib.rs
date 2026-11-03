//! 青简剪贴板的设备间同步，不经过任何服务器。打开同步时才建 [`SyncService`]，平台只提供 [`Replica`]。
//!
//! 通道只要求一条可靠的字节流（[`std::io::Read`] + [`std::io::Write`]），换一种传输方式时配对、加密与合并都不用改：
//! - 配对（[`channel::pair_host`]、[`channel::pair_join`]）：一台显示 6 位配对码，另一台输入。码经 SPAKE2 换出密钥，猜错就作废，抓包也算不出来。
//! - 通信（[`channel::connect`]、[`channel::accept`]）：Noise XX 加密，对方的公钥不在已配对名单里就断开。
//! - 合并（[`sync`]）：每条按内容取 id，比较修改时刻，新的覆盖旧的，删除也同步。

pub mod channel;
pub mod default_name;
pub mod discovery;
pub mod error;
pub mod identity;
pub mod peers;
pub mod record;
pub mod service;
pub mod sync;

pub use error::SyncError;
pub use identity::Identity;
pub use peers::{Peer, PeerBook};
pub use record::{Record, RecordKind, Stamp};
pub use service::{
    Nearby, PeerState, PeerStatus, ReceivedItem, SyncEvent, SyncOptions, SyncService,
};
pub use sync::Replica;
