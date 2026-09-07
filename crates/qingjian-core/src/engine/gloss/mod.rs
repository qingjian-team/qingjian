//! 释义兜底：随包释义表查不到的词，上屏后交给 [`GlossFiller`] 在后台问云端写一条释义，
//! 结果经 [`super::Translator::learn`] 进个人释义表，下次候选里就有译词。
//!
//! 与联想（`Predictor`）分开走：联想是「最新请求优先」、防抖会丢旧请求，兜底恰恰要「每个词都问到、慢点没关系」。
//! 接口同样非阻塞：`request` 只入队，结果由壳定时 `Engine::poll_glosses` 取。只在云联想开着时才有实现接进来
//! （发出去的只有那个词本身），缺省 [`NoGlossFiller`] 什么都不问。

mod filled_gloss;
mod filler;

pub use filled_gloss::FilledGloss;
pub use filler::{GlossFiller, NoGlossFiller};
