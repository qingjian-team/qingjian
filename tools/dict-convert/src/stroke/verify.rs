//! 抽样对照：产物的笔画数逐个对大陆规范，白名单之外一处不符即失败。
//!
//! 抽样 = 字表（缺省一级字表 `level1_3500.tsv`）按表序每 `stride` 字取一个；对照表的笔画数来自
//! Make Me a Hanzi（PRC 笔顺，见 `assets/stroke/README.md`），只作开发期校验、不随包分发。
//! 白名单（`assets/stroke/residual-whitelist.tsv`）里是已知且接受的差异：CNS 台湾标准字形与大陆规范
//! 在整字 / 部件层面就不同、不能由部件规则推出来的那些字。

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::error::ConvertError;
use crate::stroke::rules::{malformed, single_char};

/// 比对抽样表里的字：返回（抽样数、命中的白名单字、白名单之外的不符数）。
pub(super) fn compare(
    sampled: &[char],
    produced: &HashMap<char, usize>,
    expected: &HashMap<char, usize>,
    whitelist: &HashSet<char>,
) -> (usize, HashSet<char>, usize) {
    let mut used = HashSet::new();
    let mut unmatched = 0usize;
    for ch in sampled {
        let Some(want) = expected.get(ch) else {
            tracing::warn!(character = %ch, "对照表没有这个字，跳过");
            continue;
        };
        let got = produced.get(ch).copied();
        if got == Some(*want) {
            continue;
        }
        if whitelist.contains(ch) {
            used.insert(*ch);
            tracing::info!(character = %ch, table = ?got, reference = want, "白名单内的已知差异");
            continue;
        }
        unmatched += 1;
        tracing::error!(character = %ch, table = ?got, reference = want, "白名单之外的不符");
    }
    (sampled.len(), used, unmatched)
}

/// 按表序每 `stride` 字取一个。
pub(super) fn sample(chars: &[char], stride: usize) -> Vec<char> {
    chars
        .iter()
        .enumerate()
        .filter(|(index, _)| (index + 1) % stride == 0)
        .map(|(_, ch)| *ch)
        .collect()
}

/// 跑一次抽样对照，白名单之外有不符合就报错。
pub fn run(
    table: &Path,
    sample_table: &Path,
    stride: usize,
    reference: &Path,
    whitelist: &Path,
) -> Result<(), ConvertError> {
    if stride == 0 {
        return Err(malformed(
            sample_table,
            0,
            "stride must be greater than zero",
        ));
    }
    let produced = read_lengths(table)?;
    let expected = read_reference(reference)?;
    let chars = read_chars(sample_table)?;
    let known = read_whitelist(whitelist)?;
    let sampled = sample(&chars, stride);
    let (count, used, unmatched) = compare(&sampled, &produced, &expected, &known);
    tracing::info!(
        table = %table.display(),
        sampled = count,
        reference_entries = expected.len(),
        whitelisted = used.len(),
        unmatched,
        "抽样对照完成"
    );
    if used.len() < known.len() {
        tracing::warn!(
            unused = known.len() - used.len(),
            path = %whitelist.display(),
            "白名单里有字这次没有不符，可以删掉"
        );
    }
    if unmatched > 0 {
        return Err(ConvertError::Verify { unmatched });
    }
    Ok(())
}

/// 读产物：字 → 序列长度。
fn read_lengths(path: &Path) -> Result<HashMap<char, usize>, ConvertError> {
    let mut lengths = HashMap::new();
    for (index, line) in BufReader::new(File::open(path)?).lines().enumerate() {
        let line = line?;
        let number = index + 1;
        if line.is_empty() {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(ch), Some(sequence)) = (fields.next(), fields.next()) else {
            return Err(malformed(
                path,
                number,
                "expected a character and a stroke sequence",
            ));
        };
        let Some(ch) = single_char(ch) else {
            return Err(malformed(path, number, "expected one character"));
        };
        if sequence.is_empty() {
            return Err(malformed(path, number, "stroke sequence is empty"));
        }
        lengths.insert(ch, sequence.chars().count());
    }
    Ok(lengths)
}

/// 读对照表：字 → 大陆笔画数。
fn read_reference(path: &Path) -> Result<HashMap<char, usize>, ConvertError> {
    let mut counts = HashMap::new();
    for (index, line) in BufReader::new(File::open(path)?).lines().enumerate() {
        let line = line?;
        let number = index + 1;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(ch), Some(value)) = (fields.next(), fields.next()) else {
            return Err(malformed(
                path,
                number,
                "expected a character and a stroke count",
            ));
        };
        let Some(ch) = single_char(ch) else {
            return Err(malformed(path, number, "expected one character"));
        };
        let count = value
            .parse::<usize>()
            .map_err(|_| malformed(path, number, "stroke count is not a number"))?;
        counts.insert(ch, count);
    }
    Ok(counts)
}

/// 读抽样字表：一行一个字（第一列），表序即抽样序。
fn read_chars(path: &Path) -> Result<Vec<char>, ConvertError> {
    let mut chars = Vec::new();
    let mut seen = HashSet::new();
    for line in BufReader::new(File::open(path)?).lines() {
        let line = line?;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(ch) = single_char(line.split('\t').next().unwrap_or_default()) else {
            continue;
        };
        if seen.insert(ch) {
            chars.push(ch);
        }
    }
    Ok(chars)
}

/// 读白名单里的字（第一列）。
fn read_whitelist(path: &Path) -> Result<HashSet<char>, ConvertError> {
    Ok(read_chars(path)?.into_iter().collect::<HashSet<char>>())
}
