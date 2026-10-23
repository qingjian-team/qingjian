//! 连 Server 的客户端：一条 Unix socket，一问一答。
//!
//! 与 Fcitx5 插件（C++，`apps/linux/fcitx5/src/ipc/`）是同一个协议的两份实现，
//! 线上类型在 `qingjian_platform::protocol::linux`，两边都从那里取。

mod connection;
mod reply;
mod shared;

#[cfg(test)]
mod tests;

pub use connection::Connection;
pub use reply::Reply;
pub use shared::Shared;
