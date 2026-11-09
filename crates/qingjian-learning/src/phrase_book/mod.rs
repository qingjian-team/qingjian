//! 常用语（`phrases.json`）：分组存的一条条短语，可带输入码（打这串码出这条，换成 Core 的 [`qingjian_core::CustomPhrase`]）。
//! 增删改、移到最前、分组都在这里，各平台共用；壳只管列出来、编辑与上屏。

mod book;
mod phrase;

pub use book::{DEFAULT_GROUP, MAX_CODE_CHARS, PHRASE_BOOK_FILE, PhraseBook};
pub use phrase::Phrase;
