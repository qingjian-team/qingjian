//! IBus 接入：把 Server 回的帧交给 IBus 自带的面板画。
//!
//! GNOME 的 Wayland 会话里内建的就是 IBus，装了 fcitx5 的机器则走 `apps/linux/fcitx5`；
//! 两个前端连同一个 Server，用户实际用哪个由 `$XMODIFIERS` 决定。
//!
//! 分两层：[`variant`] 是 IBus 那套 GVariant 对象的序列化，[`view`] 把协议里的
//! [`Frame`](qingjian_platform::protocol::Frame) 折成「preedit 什么样、候选表什么样、辅助行写什么」。
//! D-Bus 服务在 [`engine`] 与 [`factory`]，总线地址的找法在 [`service`]。
//!
//! Linux Server 不推送（协议是严格的一问一答）：帧大多是某次事件的回包；本地整句模型的重排结果
//! 要前端在组句期间自己来取，那一层在 [`poller`]，与 Fcitx5 插件的定时 `Poll` 同一个做法。

pub mod component;
pub mod engine;
pub mod factory;
pub mod poller;
pub mod service;
pub mod variant;
pub mod view;
