//! 知意 `tokenizer.json` 的分词（BPE + 字节兜底），只做编码。
//!
//! 照 `tokenizers` 对这份配置的行为：先按附加词（`added_tokens`，取最左最长，`lstrip` 吞掉左边的空白）切开，
//! 其余各段空格换成 `▁`、段首不是 `▁` 就补一个、在每个 `▁` 前断开，逐段从单字起按合并表的名次合并；
//! 词表里没有的字拆成 `<0xXX>` 字节。不用 `tokenizers`：它给 25.6 万词、58 万条合并规则建的通用结构常驻与加载峰值都大得多
//! （数字见 crate-notes），这里只留编码要的两张表。与 `tokenizers` 对拍见 `tests.rs`。

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::NeuralError;

const SPACE: char = '▁';

pub(super) struct Bpe {
    vocab: HashMap<Box<str>, u32>,

    /// 相邻两个 token → (名次, 合并成的 token)，名次小的先合并。
    merges: HashMap<(u32, u32), (u32, u32)>,

    /// 附加词按首字分组，同组长的在前。
    added: HashMap<char, Vec<AddedToken>>,

    unk: Option<u32>,
}

struct AddedToken {
    content: Box<str>,

    id: u32,

    lstrip: bool,
}

#[derive(Deserialize)]
struct File<'a> {
    #[serde(borrow)]
    added_tokens: Vec<FileAddedToken<'a>>,

    #[serde(borrow)]
    model: FileModel<'a>,
}

#[derive(Deserialize)]
struct FileAddedToken<'a> {
    id: u32,

    #[serde(borrow)]
    content: Piece<'a>,

    #[serde(default)]
    lstrip: bool,
}

#[derive(Deserialize)]
struct FileModel<'a> {
    #[serde(borrow)]
    vocab: HashMap<Piece<'a>, u32>,

    #[serde(borrow)]
    merges: Vec<(Piece<'a>, Piece<'a>)>,

    #[serde(default)]
    unk_token: Option<Piece<'a>>,
}

/// 没有转义的字符串直接借文件里的，省下百万次小分配。
#[derive(Deserialize, PartialEq, Eq, Hash)]
struct Piece<'a>(#[serde(borrow)] Cow<'a, str>);

impl Bpe {
    pub fn load(path: &Path) -> Result<Self, NeuralError> {
        let text = std::fs::read_to_string(path).map_err(|source| NeuralError::Io {
            path: path.to_owned(),
            source,
        })?;
        let file: File = serde_json::from_str(&text).map_err(|source| NeuralError::Json {
            path: path.to_owned(),
            source,
        })?;
        let vocab: HashMap<Box<str>, u32> = file
            .model
            .vocab
            .into_iter()
            .map(|(piece, id)| (Box::from(piece.0.as_ref()), id))
            .collect();
        let mut merges = HashMap::with_capacity(file.model.merges.len());
        for (rank, (left, right)) in file.model.merges.iter().enumerate() {
            let id = |text: &str| vocab.get(text).copied();
            let (Some(l), Some(r), Some(merged)) = (
                id(&left.0),
                id(&right.0),
                id(&format!("{}{}", left.0, right.0)),
            ) else {
                return Err(NeuralError::Corrupt("tokenizer.json merge outside vocab"));
            };
            // 重复的规则后一条为准（与 `tokenizers` 建表一致）
            merges.insert((l, r), (rank as u32, merged));
        }
        let mut added: HashMap<char, Vec<AddedToken>> = HashMap::new();
        for token in file.added_tokens {
            let Some(first) = token.content.0.chars().next() else {
                continue;
            };
            added.entry(first).or_default().push(AddedToken {
                content: Box::from(token.content.0.as_ref()),
                id: token.id,
                lstrip: token.lstrip,
            });
        }
        for group in added.values_mut() {
            group.sort_by_key(|token| std::cmp::Reverse(token.content.len()));
        }
        let unk = file
            .model
            .unk_token
            .and_then(|name| vocab.get(name.0.as_ref()).copied());
        Ok(Self {
            vocab,
            merges,
            added,
            unk,
        })
    }

    pub fn encode(&self, text: &str) -> Vec<u32> {
        let mut ids = Vec::new();
        let mut rest = 0;
        while let Some((start, end, id)) = self.next_added(text, rest) {
            self.encode_segment(&text[rest..start], &mut ids);
            ids.push(id);
            rest = end;
        }
        self.encode_segment(&text[rest..], &mut ids);
        ids
    }

    /// `from` 起第一个附加词：(含 `lstrip` 吞掉的空白在内的起点, 终点, token)。
    fn next_added(&self, text: &str, from: usize) -> Option<(usize, usize, u32)> {
        for (offset, c) in text[from..].char_indices() {
            let at = from + offset;
            let Some(token) = self
                .added
                .get(&c)
                .and_then(|group| group.iter().find(|t| text[at..].starts_with(&*t.content)))
            else {
                continue;
            };
            let start = if token.lstrip {
                text[from..at]
                    .char_indices()
                    .rev()
                    .take_while(|(_, c)| c.is_whitespace())
                    .last()
                    .map_or(at, |(i, _)| from + i)
            } else {
                at
            };
            return Some((start, at + token.content.len(), token.id));
        }
        None
    }

    fn encode_segment(&self, segment: &str, ids: &mut Vec<u32>) {
        if segment.is_empty() {
            return;
        }
        let mut normalized = segment.replace(' ', "▁");
        if !normalized.starts_with(SPACE) {
            normalized.insert(0, SPACE);
        }
        let mut word_start = 0;
        for (i, c) in normalized.char_indices() {
            if c == SPACE && i > word_start {
                self.encode_word(&normalized[word_start..i], ids);
                word_start = i;
            }
        }
        self.encode_word(&normalized[word_start..], ids);
    }

    fn encode_word(&self, word: &str, ids: &mut Vec<u32>) {
        let mut symbols: Vec<u32> = Vec::with_capacity(word.len());
        let mut buffer = [0u8; 4];
        for c in word.chars() {
            if let Some(&id) = self.vocab.get(c.encode_utf8(&mut buffer) as &str) {
                symbols.push(id);
                continue;
            }
            let bytes: Option<Vec<u32>> = c
                .encode_utf8(&mut buffer)
                .bytes()
                .map(|b| self.vocab.get(format!("<0x{b:02X}>").as_str()).copied())
                .collect();
            match bytes {
                Some(bytes) => symbols.extend(bytes),
                None => {
                    if let Some(unk) = self.unk
                        && symbols.last() != Some(&unk)
                    {
                        symbols.push(unk);
                    }
                }
            }
        }
        while symbols.len() > 1 {
            let best = symbols
                .windows(2)
                .enumerate()
                .filter_map(|(i, pair)| Some((self.merges.get(&(pair[0], pair[1]))?, i)))
                .min_by_key(|&(&(rank, _), i)| (rank, i));
            let Some((&(_, merged), i)) = best else {
                break;
            };
            symbols[i] = merged;
            symbols.remove(i + 1);
        }
        ids.extend(symbols);
    }
}
