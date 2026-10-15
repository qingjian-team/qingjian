//! Linux 独立的面板协议扩展；Windows 的 `ClientMessage` / `ServerMessage` 与版本号不变。
//!
//! 与 Windows 那套同样的道理（见父模块文档）：Server 进程扛 [`qingjian_core::Engine`]，
//! 各框架的前端（Fcitx5 addon、IBus 前端）只做事件翻译与贴图，所以线上类型必须放在
//! **两端都能依赖、又不拖进 Engine 依赖树**的这里。

mod capabilities;
mod display;
mod event;
mod identity;
mod request;
mod socket;

pub use capabilities::Capabilities;
pub use display::DisplayAcknowledged;
pub use event::LinuxEvent;
pub use identity::DisplayIdentity;
pub use request::LinuxRequest;
pub use socket::socket_path;

/// Linux 面板扩展的版本，前端在 `LinuxHello` 里带上，对不上 Server 直接断开。
/// 与 [`super::PROTOCOL_VERSION`] 各自独立：那个管 Windows 那套消息，这个管 Linux 这几种事件。
pub const LINUX_UI_PROTOCOL: u32 = 3;
