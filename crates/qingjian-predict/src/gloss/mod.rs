//! 释义兜底：[`qingjian_core::GlossFiller`] 的网络实现。
//!
//! 独立线程、独立通道，与联想互不干扰：把主线程送来的词攒成一小批（等 [`worker::BATCH_WAIT`] 或攒够
//! [`worker::BATCH_SIZE`]），一次请求让模型给每个词写 1–2 条学习语言的译词（提示词与 `gloss-gen` 同源，
//! 只要当前学习语言），解析、清洗后逐条回给主线程。问过的词本进程内不再问，失败也不重试（下次上屏再说）。

mod cloud_gloss_filler;
mod prompt;
mod worker;

pub use cloud_gloss_filler::CloudGlossFiller;
