//! 通讯录词库：名字 → 读音的小词库，加上每个名字上屏后接着给的内容。

use std::collections::HashMap;

use qingjian_dictionary::Dictionary;

use super::contact::Contact;
use super::reading::readings;
use crate::sentence::is_han;

/// 名字最长几个字：再长的多半是备注（「王师傅 修空调」），不当名字。
const MAX_NAME_CHARS: usize = 6;

/// 一个名字最多接几条内容（电话、邮箱）：候选条一屏放得下。
const MAX_DETAILS: usize = 6;

/// 名字在词库里的词频：只够进整句词图，排序靠查词后把整段对得上的名字挪到第一页（`Engine` 那边）。
const NAME_FREQUENCY: u32 = 1000;

pub struct ContactBook {
    /// 名字的词库，与主词库一起查词。
    dictionary: Dictionary,

    /// 名字 → 上屏后接着给的内容，同名的联系人合在一起。
    details: HashMap<String, Vec<String>>,
}

impl ContactBook {
    /// 从联系人建：名字去掉空白后须是 2–6 个汉字，其余（英文名、带符号的备注）跳过。
    /// `word` 问某组音节下词库里有没有这个名字，`fallback` 给单字表里没有的字一个读音。
    pub fn build(
        contacts: &[Contact],
        mut word: impl FnMut(&str, &[&str]) -> bool,
        mut fallback: impl FnMut(char) -> Option<String>,
    ) -> Self {
        let mut details: HashMap<String, Vec<String>> = HashMap::new();
        let mut tsv = String::new();
        for contact in contacts {
            let name: String = contact.name.split_whitespace().collect();
            let count = name.chars().count();
            if !(2..=MAX_NAME_CHARS).contains(&count) || !name.chars().all(is_han) {
                continue;
            }
            if !details.contains_key(&name) {
                let found = readings(&name, |syllables| word(&name, syllables), &mut fallback);
                if found.is_empty() {
                    continue;
                }
                for reading in found {
                    tsv.push_str(&format!("{name}\t{reading}\t{NAME_FREQUENCY}\n"));
                }
            }
            let list = details.entry(name).or_default();
            for detail in &contact.details {
                let detail = detail.trim();
                if !detail.is_empty()
                    && list.len() < MAX_DETAILS
                    && !list.iter().any(|d| d == detail)
                {
                    list.push(detail.to_owned());
                }
            }
        }
        let dictionary = Dictionary::parse(&tsv).unwrap_or_else(|error| {
            tracing::warn!(%error, "contact dictionary failed to build");
            Dictionary::parse("").expect("an empty dictionary always parses")
        });
        Self {
            dictionary,
            details,
        }
    }

    pub fn dictionary(&self) -> &Dictionary {
        &self.dictionary
    }

    /// 收进来的名字有几个。
    pub fn len(&self) -> usize {
        self.details.len()
    }

    pub fn is_empty(&self) -> bool {
        self.details.is_empty()
    }

    /// `text` 是不是通讯录里的名字。
    pub fn contains(&self, text: &str) -> bool {
        self.details.contains_key(text)
    }

    /// 名字上屏后接着给的内容；不是名字或没有内容时为空。
    pub fn details(&self, name: &str) -> &[String] {
        self.details.get(name).map_or(&[], Vec::as_slice)
    }

    /// `text` 里有没有通讯录里的名字。
    pub fn mentioned_in(&self, text: &str) -> bool {
        !text.is_empty() && self.details.keys().any(|name| text.contains(name.as_str()))
    }
}
