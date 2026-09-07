//! 读「输入法字词库_分类整理版」目录：`01_characters/standard_8105.tsv`、`02_common/modern_chinese_common_words.tsv`、`03_domains/*.tsv`。
//! 三种 TSV 首行都是字段名：`词条\t拼音\t排序号\t文档频次\t字表级别\t来源`。

use std::path::Path;

use super::tone::numeric_syllables;
use crate::error::ConvertError;

/// 规范字表的一行。
#[derive(Debug, Clone)]
pub struct CharRow {
    /// 字。
    pub ch: char,

    /// 字表级别 1–3。
    pub level: u8,
}

/// 通用词表的一行。
#[derive(Debug, Clone)]
pub struct CommonRow {
    /// 词。
    pub text: String,

    /// 音节（已规范化）。
    pub syllables: Vec<String>,

    /// 原词号，越小越常用。
    pub rank: u32,
}

/// 领域词表的一行。
#[derive(Debug, Clone)]
pub struct DomainRow {
    /// 词。
    pub text: String,

    /// 文档频次（THUOCL DF），缺失为 0。
    pub df: u64,

    /// 来自 `03_domains/` 的哪个文件（文件名主干，如 `law`）；语料挖出的额外词为 `None`，一律留在基础词库。
    pub domain: Option<String>,
}

/// 整个数据包。
#[derive(Debug, Default)]
pub struct Pack {
    pub chars: Vec<CharRow>,

    pub common: Vec<CommonRow>,

    pub domain: Vec<DomainRow>,
}

impl Pack {
    pub fn load(dir: &Path) -> Result<Self, ConvertError> {
        let mut pack = Self::default();
        for line in rows(&dir.join("01_characters/standard_8105.tsv"))? {
            let fields: Vec<&str> = line.split('\t').collect();
            let Some(ch) = fields.first().and_then(|f| clean(f).chars().next()) else {
                continue;
            };
            let level = fields
                .get(4)
                .and_then(|f| f.trim().parse().ok())
                .unwrap_or(3);
            pack.chars.push(CharRow { ch, level });
        }
        for line in rows(&dir.join("02_common/modern_chinese_common_words.tsv"))? {
            let fields: Vec<&str> = line.split('\t').collect();
            let (Some(text), Some(pinyin)) = (fields.first(), fields.get(1)) else {
                continue;
            };
            let text = clean(text);
            let syllables = numeric_syllables(pinyin);
            // 拼音与字数对不上的（阿Ｑ、带逗号的成语）不要
            if text.is_empty()
                || syllables.len() != text.chars().count()
                || !text.chars().all(is_han)
            {
                continue;
            }
            let rank = fields
                .get(2)
                .and_then(|f| f.trim().parse().ok())
                .unwrap_or(u32::MAX);
            pack.common.push(CommonRow {
                text,
                syllables,
                rank,
            });
        }
        let domain_dir = dir.join("03_domains");
        let mut files: Vec<_> = std::fs::read_dir(&domain_dir)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "tsv"))
            .collect();
        files.sort();
        for file in files {
            let before = pack.domain.len();
            let stem = file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_owned();
            for line in rows(&file)? {
                let fields: Vec<&str> = line.split('\t').collect();
                let Some(text) = fields.first().map(|f| clean(f)) else {
                    continue;
                };
                if text.is_empty() || !text.chars().all(is_han) {
                    continue;
                }
                let df = fields
                    .get(3)
                    .and_then(|f| f.trim().parse().ok())
                    .unwrap_or(0);
                pack.domain.push(DomainRow {
                    text,
                    df,
                    domain: Some(stem.clone()),
                });
            }
            tracing::info!(file = %file.display(), rows = pack.domain.len() - before, "领域词已读取");
        }
        Ok(pack)
    }
}

/// 读一个带表头的 TSV，跳过首行、空行。
fn rows(path: &Path) -> Result<Vec<String>, ConvertError> {
    let source = std::fs::read_to_string(path)?;
    Ok(source
        .lines()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .map(str::to_owned)
        .collect())
}

/// 去掉 BOM 与首尾空白。
fn clean(field: &str) -> String {
    field
        .trim()
        .trim_start_matches('\u{feff}')
        .trim()
        .to_owned()
}

/// 汉字（基本区 + 扩展 A）。
fn is_han(c: char) -> bool {
    matches!(c, '\u{4e00}'..='\u{9fff}' | '\u{3400}'..='\u{4dbf}')
}
