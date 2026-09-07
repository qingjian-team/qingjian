//! 输入日志：每次上屏记一条「敲了什么、看到了什么、选了什么」，退格撤销也记一条。
//!
//! 聚合的学习数据（选择次数、输入串选择、个人 n-gram）回答不了这个问题，而排序 / 整句的离线回归评测
//! 与个人模型的训练对都要它。Core 只产生条目（[`InputLogEntry`]），写到哪、加不加时间戳由
//! [`InputLogger`] 的实现定（`qingjian-learning` 写本机 jsonl）；缺省 [`NoInputLogger`] 什么都不记。
//! 条目里只有用户敲的键与上屏的文字，不含应用里的上下文。

mod entry;
mod logger;
mod source;

pub use entry::{CommitEntry, InputLogEntry};
pub use logger::{InputLogger, NoInputLogger};
pub use source::InputSource;

/// 写进条目的候选文本最多几条：够算「首选命中了没有」和「选的是第几个」，又不把整页都抄一遍。
pub const LOGGED_CANDIDATES: usize = 5;
