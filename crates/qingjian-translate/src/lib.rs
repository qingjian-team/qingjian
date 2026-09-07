//! 候选翻译 annotation：本地 gloss 表查询。
//!
//! 运行时不联网。文件格式为 TSV，一行一个词，后面每列一条释义，最多取两条；
//! 释义开头可带词性缩写（`n.` / `v.` / `adj.` …，后跟空格），没有也行：
//!
//! ```text
//! 词\t[词性. ]译文[\t[词性. ]译文]
//! 开发\tv. develop\tn. development
//! 中文\tChinese language
//! ```
//!
//! `#` 开头为注释行。每个文件对应一种学习语言，语言由构造时指定。
//!
//! [`LevelTable`] 是词汇等级表（`levels-<语言>.tsv`，`词\t等级`），给词汇统计按级数词用。
//! [`PersonalGlossary`] 是用户目录里的个人释义表（释义兜底写入、可手改），[`LayeredTranslator`] 把它叠在随包表上面。

mod error;
mod glossary;
mod layered_translator;
mod level_table;
mod personal_glossary;

pub use error::GlossaryError;
pub use glossary::Glossary;
pub use layered_translator::LayeredTranslator;
pub use level_table::LevelTable;
pub use personal_glossary::PersonalGlossary;
