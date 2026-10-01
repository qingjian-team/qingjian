//! 可同时启用的额外翻页键；只接受已提供的三组符号，不改变主翻页键设置。

use serde::{Deserialize, Serialize};

use super::PAGE_KEY_OPTIONS;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<String>", into = "Vec<String>")]
pub struct PageKeys(u8);

impl PageKeys {
    pub fn contains(self, pair: &str) -> bool {
        PAGE_KEY_OPTIONS
            .iter()
            .position(|value| *value == pair)
            .is_some_and(|index| self.0 & (1 << index) != 0)
    }

    pub fn with(mut self, pair: &str, enabled: bool) -> Self {
        if let Some(index) = PAGE_KEY_OPTIONS.iter().position(|value| *value == pair) {
            if enabled {
                self.0 |= 1 << index;
            } else {
                self.0 &= !(1 << index);
            }
        }
        self
    }

    pub fn step(self, character: char) -> Option<isize> {
        PAGE_KEY_OPTIONS
            .iter()
            .filter(|pair| self.contains(pair))
            .find_map(|pair| {
                let mut chars = pair.chars();
                if chars.next() == Some(character) {
                    Some(-1)
                } else if chars.next() == Some(character) {
                    Some(1)
                } else {
                    None
                }
            })
    }
}

impl TryFrom<Vec<String>> for PageKeys {
    type Error = String;

    fn try_from(values: Vec<String>) -> Result<Self, Self::Error> {
        let mut keys = Self::default();
        for value in values {
            if !PAGE_KEY_OPTIONS.contains(&value.as_str()) {
                return Err(format!("unknown page key pair: {value:?}"));
            }
            keys = keys.with(&value, true);
        }
        Ok(keys)
    }
}

impl From<PageKeys> for Vec<String> {
    fn from(keys: PageKeys) -> Self {
        PAGE_KEY_OPTIONS
            .iter()
            .filter(|pair| keys.contains(pair))
            .map(|pair| (*pair).to_owned())
            .collect()
    }
}
