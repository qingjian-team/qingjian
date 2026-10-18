//! 输入统计：每次上屏折成几个汉字、几个中文词、几个英文词，选的是不是首选、敲了几键、打了多久，交给 [`UsageMeter`] 按小时累计。
//!
//! 这是给用户看的「用青简打了多少字」，不是学习数据，也不从输入日志算（日志可以关、可以清，统计还在）。
//! Core 只负责算数（[`Usage::of_text`] 与 `Engine` 里的词数、[`Typing`] 的键数与时长），按小时累计与落盘由实现做
//! （`qingjian-learning` 的 `UsageStats`）；缺省 [`NoUsageMeter`] 什么都不记。

mod book;
mod meter;
mod summary;
mod typing;
mod usage;

pub use book::{BOOKS, Book, book_scale};
pub use meter::{NoUsageMeter, UsageMeter};
pub use summary::UsageSummary;
pub(super) use typing::Typing;
pub use usage::Usage;
