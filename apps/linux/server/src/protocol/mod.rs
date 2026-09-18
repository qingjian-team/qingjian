//! Linux 独立的面板协议扩展；Windows ClientMessage/ServerMessage 和版本不变。
mod capabilities;
mod display;
mod event;
mod identity;
mod request;
pub use capabilities::Capabilities;
pub use display::DisplayAcknowledged;
pub use event::LinuxEvent;
pub use identity::DisplayIdentity;
pub use request::LinuxRequest;
pub const LINUX_UI_PROTOCOL: u32 = 2;
