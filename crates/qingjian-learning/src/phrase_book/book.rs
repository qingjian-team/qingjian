//! [`PhraseBook`]：读写、增删改、换成自定义短语。

use std::path::{Path, PathBuf};

use qingjian_core::CustomPhrase;
use qingjian_core::custom_phrase::{MAX_CODE_LEN, valid_code};
use qingjian_core::storage::{read_text_lossy, write_atomic_str};

use super::Phrase;
use crate::error::LearningError;

pub const PHRASE_BOOK_FILE: &str = "phrases.json";

/// 没指定分组时放进去的那组，总排第一。
pub const DEFAULT_GROUP: &str = "默认";

/// 最多存多少条。
const MAX_PHRASES: usize = 10_000;

/// 输入码最长几个字母，跟 Core 自定义短语的上限一致：长了挂不成短语。
pub const MAX_CODE_CHARS: usize = MAX_CODE_LEN;

#[derive(Debug, Default)]
pub struct PhraseBook {
    /// 越靠前越常用（新加的、移到最前的在前）。
    phrases: Vec<Phrase>,

    /// `None` 只在内存里记。
    path: Option<PathBuf>,
}

impl PhraseBook {
    /// 读数据目录里的常用语（不存在就是空的）；读不了或格式坏了就退回只在内存里记，不拿空表覆盖用户文件。
    pub fn open(dir: &Path) -> Self {
        let path = dir.join(PHRASE_BOOK_FILE);
        match read_text_lossy(&path) {
            Ok(None) => Self {
                phrases: Vec::new(),
                path: Some(path),
            },
            Ok(Some(text)) => match serde_json::from_str(&text) {
                Ok(phrases) => Self {
                    phrases,
                    path: Some(path),
                },
                Err(error) => {
                    tracing::warn!(path = %path.display(), %error, "常用语文件格式不对，本次只在内存里记");
                    Self::default()
                }
            },
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "常用语读不了，本次只在内存里记");
                Self::default()
            }
        }
    }

    pub fn phrases(&self) -> &[Phrase] {
        &self.phrases
    }

    /// 出现过的分组，[`DEFAULT_GROUP`] 总在第一个，其余按第一次出现的顺序。
    pub fn groups(&self) -> Vec<String> {
        let mut groups = vec![DEFAULT_GROUP.to_owned()];
        for phrase in &self.phrases {
            if !groups.contains(&phrase.group) {
                groups.push(phrase.group.clone());
            }
        }
        groups
    }

    /// 加一条放到最前；文字为空、或已满不加。返回有没有加。
    pub fn add(&mut self, text: &str, group: &str, code: &str) -> Result<bool, LearningError> {
        let Some(phrase) = normalize(text, group, code) else {
            return Ok(false);
        };
        if self.phrases.len() >= MAX_PHRASES {
            return Ok(false);
        }
        self.phrases.insert(0, phrase);
        self.save()?;
        Ok(true)
    }

    /// 改第 `index` 条（位置不变）；越界或文字为空不改。返回有没有改。
    pub fn update(
        &mut self,
        index: usize,
        text: &str,
        group: &str,
        code: &str,
    ) -> Result<bool, LearningError> {
        let (Some(slot), Some(phrase)) =
            (self.phrases.get_mut(index), normalize(text, group, code))
        else {
            return Ok(false);
        };
        *slot = phrase;
        self.save()?;
        Ok(true)
    }

    pub fn remove(&mut self, index: usize) -> Result<(), LearningError> {
        if index >= self.phrases.len() {
            return Ok(());
        }
        self.phrases.remove(index);
        self.save()
    }

    /// 第 `index` 条挪到最前（「移至首位」）。
    pub fn move_to_top(&mut self, index: usize) -> Result<(), LearningError> {
        if index == 0 || index >= self.phrases.len() {
            return Ok(());
        }
        let phrase = self.phrases.remove(index);
        self.phrases.insert(0, phrase);
        self.save()
    }

    /// 带输入码的常用语换成 Core 的自定义短语：权重都一样，同一个码按常用语的先后排。
    /// 以前存下的超长输入码挂不成短语，跳过它，不连累别的。
    pub fn custom_phrases(&self) -> Vec<CustomPhrase> {
        let mut phrases: Vec<CustomPhrase> = Vec::new();
        for phrase in self
            .phrases
            .iter()
            .filter(|phrase| valid_code(&phrase.code))
        {
            if !phrases
                .iter()
                .any(|p| p.code == phrase.code && p.text == phrase.text)
            {
                phrases.push(CustomPhrase::new(&phrase.code, &phrase.text));
            }
        }
        phrases
    }

    fn save(&self) -> Result<(), LearningError> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let text = serde_json::to_string_pretty(&self.phrases).expect("常用语总能序列化");
        write_atomic_str(path, &text)?;
        Ok(())
    }
}

/// 文字为空返回 `None`；分组空了归 [`DEFAULT_GROUP`]；输入码只留字母、转小写、截到 [`MAX_CODE_CHARS`]。
fn normalize(text: &str, group: &str, code: &str) -> Option<Phrase> {
    if text.trim().is_empty() {
        return None;
    }
    let group = group.trim();
    Some(Phrase {
        text: text.to_owned(),
        group: if group.is_empty() {
            DEFAULT_GROUP
        } else {
            group
        }
        .to_owned(),
        code: code
            .chars()
            .filter(char::is_ascii_alphabetic)
            .map(|c| c.to_ascii_lowercase())
            .take(MAX_CODE_CHARS)
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("qj-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn add_edit_move_and_remove_persist() {
        let dir = temp_dir("phrases-edit");
        let mut book = PhraseBook::open(&dir);
        assert!(book.add("你好", "", "").unwrap());
        assert!(book.add("在开会，稍后回复", "工作", "Kh1").unwrap());
        assert!(!book.add("  ", "", "").unwrap());
        assert_eq!(book.phrases()[0].code, "kh");
        assert_eq!(book.groups(), ["默认", "工作"]);
        book.move_to_top(1).unwrap();
        book.update(1, "马上到", "工作", "md").unwrap();
        book.remove(0).unwrap();
        let reopened = PhraseBook::open(&dir);
        assert_eq!(reopened.phrases().len(), 1);
        assert_eq!(reopened.phrases()[0].text, "马上到");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn codes_become_custom_phrases_in_order() {
        let mut book = PhraseBook::default();
        book.add("二", "", "aa").unwrap();
        book.add("一", "", "aa").unwrap();
        book.add("无码", "", "").unwrap();
        let custom = book.custom_phrases();
        assert_eq!(custom.len(), 2);
        assert_eq!(custom[0].text, "一");
        assert_eq!(custom[1].text, "二");
        assert!(qingjian_core::custom_phrase::validate_phrases(&custom).is_ok());
    }

    #[test]
    fn long_codes_are_cut_and_old_ones_skipped() {
        let mut book = PhraseBook::default();
        book.add("新", "", &"a".repeat(40)).unwrap();
        assert_eq!(book.phrases()[0].code.len(), MAX_CODE_CHARS);
        book.phrases.push(Phrase {
            text: "旧".to_owned(),
            group: DEFAULT_GROUP.to_owned(),
            code: "b".repeat(64),
        });
        book.add("好", "", "hh").unwrap();
        let custom = book.custom_phrases();
        assert_eq!(custom.len(), 2);
        assert!(qingjian_core::custom_phrase::validate_phrases(&custom).is_ok());
    }
}
