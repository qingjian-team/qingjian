//! 词库：按拼音音节序列查词。
//!
//! 纯数据层，不依赖任何兄弟 crate。文件格式为 TSV：
//!
//! ```text
//! 词\t音节（空格分隔）\t词频
//! 开发\tkai fa\t9000
//! ```
//!
//! `#` 开头为注释行，空行忽略。
//!
//! 内存布局面向「几十万到上百万条常驻」：词文本与拼音键各放一个连续 arena，
//! 词目只存偏移与词频，键排序后二分定位、顺序扫描前缀范围。

mod dictionary;
mod error;
pub mod import;
mod matching;
mod pattern;
mod word_list;

pub use dictionary::Dictionary;
pub use error::DictionaryError;
pub use matching::Match;
pub use pattern::SyllablePattern;
pub use word_list::WordList;
