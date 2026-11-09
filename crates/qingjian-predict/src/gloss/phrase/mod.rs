//! 整段翻译：攒一批上屏的中文，一次请求翻成学习语言（生词本的时间线用）。

mod prompt;
mod worker;

pub(super) use worker::PhraseWorker;
