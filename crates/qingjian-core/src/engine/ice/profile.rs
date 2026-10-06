//! 用户雾凇目录的一次性快照；第三方数据不随青简分发。

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::emoji::EmojiTable;
use qingjian_dictionary::{Dictionary, WordList};

pub(in crate::engine) struct Profile {
    pub(super) source: PathBuf,

    pub(in crate::engine) dictionary: Dictionary,

    pub(in crate::engine) english: WordList,

    pub(super) emoji: EmojiTable,

    pub(super) symbols: HashMap<String, Vec<String>>,

    pub(super) phrases: HashMap<String, Vec<String>>,

    pub(super) mixed: HashMap<String, Vec<String>>,

    pub(super) pins: HashMap<String, Vec<String>>,

    pub(super) radicals: Vec<(String, String, u32)>,

    pub(super) readings: HashMap<String, String>,

    pub(super) corrections: HashMap<String, (String, String)>,

    pub(super) reduced_english: HashSet<String>,

    pub(super) long_count: usize,

    pub(super) long_index: usize,

    pub(super) reduced_index: usize,

    pub(super) date_keys: HashMap<String, String>,

    pub(super) lunar_key: String,

    pub(super) lunar_template: String,

    pub(super) uuid_key: String,

    /// 同一段输入刷新时 UUID 保持不变，换输入码或上屏后才重新生成。
    pub(in crate::engine) uuid_cache: std::cell::RefCell<Option<String>>,
}
