//! 输入日志：每次上屏记一条「敲了什么、看到了什么、选了什么」，退格撤销、退格重打、直通字符、云端联想、
//! 上文断开与会话信息各记一条。格式与各事件的用途见 `docs/plan/model-eval.md`。
//!
//! 聚合的学习数据（选择次数、输入串选择、个人 n-gram）回答不了这个问题，而排序 / 整句的离线回归评测
//! 与个人模型的训练对都要它。Core 只产生条目（[`InputLogEntry`]），写到哪、加不加时间戳由
//! [`InputLogger`] 的实现定（`qingjian-learning` 写本机 jsonl）；缺省 [`NoInputLogger`] 什么都不记。
//! 条目里只有用户敲的键与上屏的文字，不含应用里的上下文。

mod entry;
mod logger;
mod muted;
mod source;

pub use entry::{CommitEntry, INPUT_LOG_VERSION, InputLogEntry};
pub use logger::{InputLogger, NoInputLogger};
pub(super) use muted::MutedLogger;
pub use source::InputSource;

/// 写进条目的候选文本最多几条：一页的量，够算「首选命中了没有」「在不在第一页」和「选的是第几个」。
pub const LOGGED_CANDIDATES: usize = 9;
