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

/// 字级字表：一个 Unicode 字符一个 token，开头几个是 `<...>` 形式的特殊 token。
#[derive(Debug, Clone)]
pub struct Vocab {
    /// 字符 → token。
    index: HashMap<char, u32>,

    /// token → 字面，解码生成结果用。
    tokens: Vec<String>,

    /// P2C 字表里分开拼音与汉字两段的 token；字级 LM 的字表没有。
    sep: Option<u32>,
}

impl Vocab {
    pub fn load(path: &Path) -> Result<Self, NeuralError> {
        let text = std::fs::read_to_string(path).map_err(|source| NeuralError::Io {
            path: path.to_owned(),
            source,
        })?;
        Self::from_json(&text, path)
    }

    /// 解析 `vocab.json` 的正文；`path` 只用来报错。
    pub(crate) fn from_json(text: &str, path: &Path) -> Result<Self, NeuralError> {
        let file: VocabFile = serde_json::from_str(text).map_err(|source| NeuralError::Json {
            path: path.to_owned(),
            source,
        })?;
        // 特殊 token 靠「不是单个字符」认，不写死个数：字级 LM 的字表有 3 个，P2C 的多一个 `<sep>`。
        let mut index = HashMap::with_capacity(file.tokens.len());
        let mut sep = None;
        for (i, token) in file.tokens.iter().enumerate() {
            let mut chars = token.chars();
            match (chars.next(), chars.next()) {
                (Some(ch), None) => {
                    index.insert(ch, i as u32);
                }
                (Some(_), Some(_)) => {
                    if token == "<sep>" {
                        sep = Some(i as u32);
                    }
                }
                (None, _) => return Err(NeuralError::Corrupt("empty vocab token")),
            }
        }
        Ok(Self {
            index,
            tokens: file.tokens,
            sep,
        })
    }

    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// P2C 的 `<sep>`；不是 P2C 字表就没有。
    pub fn sep(&self) -> Option<u32> {
        self.sep
    }

    /// token 序列 → 文本；越界的 token 跳过。
    pub fn decode(&self, ids: &[u32]) -> String {
        ids.iter()
            .filter_map(|&id| self.tokens.get(id as usize))
            .map(String::as_str)
            .collect()
    }

    /// 每个字符都在字表里。
    pub fn knows(&self, text: &str) -> bool {
        text.chars().all(|c| self.index.contains_key(&c))
    }

    /// 逐字符编码，不认识的记 [`UNK`]。
    pub fn encode(&self, text: &str) -> Vec<u32> {
        text.chars()
            .map(|c| self.index.get(&c).copied().unwrap_or(UNK))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knows_only_texts_made_of_vocab_characters() {
        let vocab = Vocab::from_json(
            r#"{"tokens": ["<pad>", "<unk>", "<eos>", "恩"]}"#,
            Path::new("vocab.json"),
        )
        .unwrap();
        assert!(vocab.knows("恩恩"));
        assert!(!vocab.knows("嗯嗯"));
        assert_eq!(vocab.encode("嗯"), [UNK]);
    }
}
