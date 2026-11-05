//! 通讯录接进 Engine：名字并进查词，整段对得上的名字挪进第一页，上屏不学、不记、不发云端。

use std::collections::{HashMap, HashSet};

use super::Engine;
use crate::candidate::{Candidate, CandidateKind};
use crate::contacts::{Contact, ContactBook};
use crate::sentence::is_han;
use crate::zhuyin::tone;

/// 整段输入对得上的名字最靠后排在第几位（从 0 数）：第一位留给原本的首选，名字紧跟在后面，
/// 不压过最常用的那个词（简拼 `zw` 撞的词很多）。
const CONTACT_SLOT: usize = 1;

impl Engine {
    /// 换上通讯录（壳读系统通讯录后交来）；空列表即摘掉。返回收进来的名字数。格子缓存随之作废。
    pub fn set_contacts(&mut self, contacts: &[Contact]) -> usize {
        self.forget_span_cache();
        if contacts.is_empty() {
            self.contacts = None;
            return 0;
        }
        let fallback = self.char_readings(contacts);
        let book = ContactBook::build(
            contacts,
            |name, syllables| {
                self.dictionary
                    .lookup(syllables, false)
                    .iter()
                    .any(|m| m.text == name)
            },
            |ch| fallback.get(&ch).cloned(),
        );
        let count = book.len();
        self.contacts = (!book.is_empty()).then_some(book);
        count
    }

    /// `text` 是不是通讯录里的名字。
    pub fn is_contact(&self, text: &str) -> bool {
        self.contacts
            .as_ref()
            .is_some_and(|book| book.contains(text))
    }

    /// 名字上屏后接着给的内容（电话、邮箱）；不是名字时为空。
    pub fn contact_details(&self, name: &str) -> &[String] {
        self.contacts
            .as_ref()
            .map_or(&[], |book| book.details(name))
    }

    pub(super) fn contact_book(&self) -> Option<&ContactBook> {
        self.contacts.as_ref()
    }

    /// `text` 里有没有通讯录里的名字（发云端前查）。
    pub(super) fn mentions_contact(&self, text: &str) -> bool {
        self.contacts
            .as_ref()
            .is_some_and(|book| book.mentioned_in(text))
    }

    /// 整段输入对得上的第一个名字排在后面时挪到第 [`CONTACT_SLOT`] 位。
    pub(super) fn lift_contact(&self, items: &mut Vec<Candidate>) {
        if self.contacts.is_none() {
            return;
        }
        let scope = self.composition.scope().len();
        let found = items.iter().position(|candidate| {
            candidate.kind == CandidateKind::Chinese
                && self.is_contact(&candidate.text)
                && self.consumed_by(candidate).0 >= scope
        });
        if let Some(index) = found
            && index > CONTACT_SLOT
        {
            let candidate = items.remove(index);
            items.insert(CONTACT_SLOT, candidate);
        }
    }

    /// 单字表里没有的字（名字里的生僻字）从主词库的单字词条取读音，词频最高的那个。
    /// 只在有这种字时扫一遍词库。
    fn char_readings(&self, contacts: &[Contact]) -> HashMap<char, String> {
        let missing: HashSet<char> = contacts
            .iter()
            .flat_map(|contact| contact.name.chars())
            .filter(|&ch| is_han(ch) && tone::readings(ch).is_empty())
            .collect();
        if missing.is_empty() {
            return HashMap::new();
        }
        let mut found: HashMap<char, (u32, String)> = HashMap::new();
        for entry in self.dictionary.entries() {
            let mut chars = entry.text.chars();
            let (Some(ch), None) = (chars.next(), chars.next()) else {
                continue;
            };
            if !missing.contains(&ch) {
                continue;
            }
            if found
                .get(&ch)
                .is_none_or(|(frequency, _)| entry.frequency > *frequency)
            {
                found.insert(ch, (entry.frequency, entry.pinyin.to_owned()));
            }
        }
        found
            .into_iter()
            .map(|(ch, (_, reading))| (ch, reading))
            .collect()
    }
}
