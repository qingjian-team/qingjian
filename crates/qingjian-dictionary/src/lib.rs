//! 词库：按拼音音节序列查词。
//!
//! 纯数据层，不依赖任何兄弟 crate。文件格式为 TSV：
//!
//! ```text
//! 词\t音节（空格分隔）\t词频
//! 开发\tkai fa\t9000
//! 开发\tkai1 fa1\t9000
//! ```
//!
//! 音节可以带数字声调（轻声写 5 或 0），查词时不看声调，声调存着给显示用（[`Tones`]）。
//!
//! `#` 开头为注释行，空行忽略。
//!
//! 内存布局面向「几十万到上百万条常驻」：词文本与拼音键各放一个连续 arena，
//! 词目只存偏移与词频，键排序后二分定位、顺序扫描前缀范围。
//!
//! 辅码码表（[`AuxCodeTable`]）是另一套数据：词 → 码的映射，按词文本排序 + 哈希索引定位（见 `aux_code_table` 模块）。
//! 另有两种旁支：形码码表 [`CodeTable`]（键是五笔编码，按前缀查）与英文词表 [`WordList`]。

#![forbid(unsafe_code)]

mod aux_code_table;
mod code_table;
mod dictionary;
mod error;
pub mod import;
mod matching;
mod pattern;
mod supplement;
mod tones;
mod word_list;

pub use aux_code_table::{
    AuxCodeLookup, AuxCodeTable, AuxCodeTableImport, AuxCodeTableImportReport, AuxCodeTableInfo,
    WordRules as AuxWordRules, aux_code_table_info, import_aux_code_table,
};
pub use code_table::CodeTable;
pub use dictionary::Dictionary;
pub use error::DictionaryError;
pub use matching::Match;
pub use pattern::{SyllablePattern, canonical_syllable};
pub use supplement::supplement;
pub use tones::{Tones, marked, split_tone, strip_marks};
pub use word_list::WordList;

/// 随包通用大词库的文件名主干（`dicts/general.qj`）：缺省打开，各壳加载时标成补充词库（[`Dictionary::set_secondary`]）。
pub const GENERAL_DICTIONARY: &str = "general";
