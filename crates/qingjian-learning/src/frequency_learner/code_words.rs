//! 按方案隔离的形码用户词，独立于拼音词库保存。

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::Path;

use qingjian_core::storage::write_atomic;
use qingjian_dictionary::CodeTable;

use crate::LearningError;

#[derive(Debug, Default)]
pub(super) struct CodeWords {
    /// （方案 id, 词）→ 编码，落盘的就是这张表。
    words: BTreeMap<(String, String), String>,

    /// 按方案 id 由 `words` 建出来的码表，查询用。
    tables: BTreeMap<String, CodeTable>,

    /// 有没落盘的改动。
    pub dirty: bool,
}

impl CodeWords {
    pub fn load(&mut self, source: &str) -> usize {
        let mut skipped = 0;
        for line in super::data_lines(source) {
            let fields: Vec<_> = line.split('\t').collect();
            if let [scheme, text, code] = fields.as_slice()
                && valid(scheme, text, code)
            {
                self.words
                    .insert(((*scheme).into(), (*text).into()), (*code).into());
            } else {
                skipped += 1;
            }
        }
        let schemes: BTreeSet<_> = self
            .words
            .keys()
            .map(|(scheme, _)| scheme.clone())
            .collect();
        for scheme in schemes {
            self.rebuild(&scheme);
        }
        skipped
    }

    pub fn learn(&mut self, scheme: &str, text: &str, code: &str) {
        if !valid(scheme, text, code) {
            return;
        }
        let key = (scheme.to_owned(), text.to_owned());
        if self
            .words
            .get(&key)
            .is_some_and(|previous| previous == code)
        {
            return;
        }
        let replaced = self.words.insert(key, code.to_owned()).is_some();
        self.dirty = true;
        if replaced {
            self.rebuild(scheme);
        } else {
            self.tables
                .entry(scheme.to_owned())
                .or_default()
                .insert_phrase(text, code, 100);
        }
    }

    pub fn table(&self, scheme: &str) -> Option<&CodeTable> {
        self.tables.get(scheme)
    }

    pub fn forget(&mut self, text: &str) -> bool {
        let schemes: Vec<_> = self
            .words
            .keys()
            .filter(|(_, word)| word == text)
            .map(|(scheme, _)| scheme.clone())
            .collect();
        for scheme in &schemes {
            self.words.remove(&(scheme.clone(), text.to_owned()));
            self.rebuild(scheme);
        }
        let removed = !schemes.is_empty();
        self.dirty |= removed;
        removed
    }

    fn rebuild(&mut self, scheme: &str) {
        let source: String = self
            .words
            .iter()
            .filter(|((key, _), _)| key == scheme)
            .map(|((_, text), code)| format!("{text}\t{code}\t100\n"))
            .collect();
        if source.is_empty() {
            self.tables.remove(scheme);
        } else if let Ok(table) = CodeTable::parse(&source) {
            self.tables.insert(scheme.to_owned(), table);
        }
    }

    pub fn save(&mut self, path: &Path) -> Result<(), LearningError> {
        write_atomic(path, |file| {
            writeln!(file, "# 青简形码用户词：方案\\t词\\t编码")?;
            for ((scheme, text), code) in &self.words {
                writeln!(file, "{scheme}\t{text}\t{code}")?;
            }
            Ok(())
        })?;
        self.dirty = false;
        Ok(())
    }
}

fn valid(scheme: &str, text: &str, code: &str) -> bool {
    !scheme.is_empty()
        && scheme.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '-' | '_')
        })
        && (2..=10).contains(&text.chars().count())
        && !text.chars().any(char::is_control)
        && !code.is_empty()
        && code.len() <= 10
        && code.chars().all(|character| character.is_ascii_lowercase())
}
