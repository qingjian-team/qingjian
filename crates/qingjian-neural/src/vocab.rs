use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::NeuralError;

/// 未登录字符的 token。
pub const UNK: u32 = 1;

/// 句首 / 行分隔 token（训练时每行末尾补它，打分时当句首用）。
pub const EOS: u32 = 2;

#[derive(Deserialize)]
struct VocabFile {
    tokens: Vec<String>,
}

/// 字级字表：一个 Unicode 字符一个 token，前三个是特殊 token。
#[derive(Debug, Clone)]
pub struct Vocab {
    /// 字符 → token。
    index: HashMap<char, u32>,

    /// token 数。
    size: usize,
}

impl Vocab {
    pub fn load(path: &Path) -> Result<Self, NeuralError> {
        let text = std::fs::read_to_string(path).map_err(|source| NeuralError::Io {
            path: path.to_owned(),
            source,
        })?;
        let file: VocabFile = serde_json::from_str(&text).map_err(|source| NeuralError::Json {
            path: path.to_owned(),
            source,
        })?;
        let mut index = HashMap::with_capacity(file.tokens.len());
        for (i, token) in file.tokens.iter().enumerate().skip(3) {
            let mut chars = token.chars();
            let (Some(ch), None) = (chars.next(), chars.next()) else {
                return Err(NeuralError::Corrupt(
                    "vocab token is not a single character",
                ));
            };
            index.insert(ch, i as u32);
        }
        Ok(Self {
            index,
            size: file.tokens.len(),
        })
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// 逐字符编码，不认识的记 [`UNK`]。
    pub fn encode(&self, text: &str) -> Vec<u32> {
        text.chars()
            .map(|c| self.index.get(&c).copied().unwrap_or(UNK))
            .collect()
    }
}
