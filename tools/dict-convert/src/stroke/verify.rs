//! 对照验收：产物逐字对大陆规范，白名单之外一处不符即失败。
//!
//! 两张对照表由 `mmh-reference` 生成、放在 `data/mmh/`（Arphic 派生，不进仓库，见 `../qingjian-dicts/official/upstream/stroke/README.md`）；
//! 找不到哪张就跳过哪张对照并提示：
//!
//! - 笔画数：按 `--stride` 抽样，比对 `--reference`；
//! - 首笔：字表全量（不抽样），比对 `--first-reference` 的几何类别。几何类别是近似（竖撇走向近竖、
//!   点的走向与撇 / 竖难分），但两岸笔顺的差异也落在这几类里，所以不按类别放行：不符的字都要在
//!   `--first-whitelist` 里。
//!
//! 白名单里是已知且接受的差异：规范裁定后保留的 CNS 结构性差异，与对照源分笔不同的假阳性。

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::error::ConvertError;
use crate::stroke::options::StrokeOptions;
use crate::stroke::rules::{malformed, single_char};

/// 首笔对照的结果计数。
pub(super) struct FirstOutcome {
    /// 有对照且可判的字数。
    pub(super) compared: usize,

    /// 产物里没有的字数（生成时被丢掉的字）。
    pub(super) absent: usize,

    /// 对照表没收的字数。
    pub(super) unreferenced: usize,

    /// 对照类别是 ?（几何上判不了）的字数。
    pub(super) incomparable: usize,

    /// 命中白名单的字。
    pub(super) used: HashSet<char>,

    /// 白名单之外的不符数。
    pub(super) unmatched: usize,
}

/// 比对抽样表里的笔画数：返回（抽样数、命中的白名单字、白名单之外的不符数）。
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

/// 比对字表全量的首笔：类别不一致又不在白名单里的记不符。
pub(super) fn compare_first(
    chars: &[char],
    produced: &HashMap<char, char>,
    expected: &HashMap<char, char>,
    whitelist: &HashSet<char>,
) -> FirstOutcome {
    let mut outcome = FirstOutcome {
        compared: 0,
        absent: 0,
        unreferenced: 0,
        incomparable: 0,
        used: HashSet::new(),
        unmatched: 0,
    };
    for ch in chars {
        let Some(ours) = produced.get(ch) else {
            outcome.absent += 1;
            tracing::warn!(character = %ch, "产物里没有这个字，跳过首笔对照");
            continue;
        };
        let Some(reference) = expected.get(ch) else {
            outcome.unreferenced += 1;
            tracing::warn!(character = %ch, "首笔对照表没有这个字，跳过");
            continue;
        };
        if *reference == '?' {
            outcome.incomparable += 1;
            continue;
        }
        if *ours == *reference {
            outcome.compared += 1;
            continue;
        }
        if whitelist.contains(ch) {
            outcome.used.insert(*ch);
            tracing::info!(character = %ch, table = %ours, reference = %reference, "白名单内的已知差异");
            continue;
        }
        outcome.unmatched += 1;
        tracing::error!(character = %ch, table = %ours, reference = %reference, "白名单之外的不符");
    }
    outcome
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

/// 跑一次对照：笔画数抽样、首笔全量，白名单之外有不符合就报错。
pub fn run(options: &StrokeOptions, table: &Path) -> Result<(), ConvertError> {
    if options.stride == 0 {
        return Err(malformed(
            &options.sample,
            0,
            "stride must be greater than zero",
        ));
    }
    let chars = read_chars(&options.sample)?;
    if options.reference.exists() {
        let produced = read_lengths(table)?;
        let expected = read_reference(&options.reference)?;
        let known = read_whitelist(&options.whitelist)?;
        let sampled = sample(&chars, options.stride);
        let (count, used, unmatched) = compare(&sampled, &produced, &expected, &known);
        tracing::info!(
            table = %table.display(),
            sampled = count,
            reference_entries = expected.len(),
            whitelisted = used.len(),
            unmatched,
            "stroke count check done"
        );
        if used.len() < known.len() {
            tracing::warn!(
                unused = known.len() - used.len(),
                path = %options.whitelist.display(),
                "whitelisted characters matched this time and can be removed"
            );
        }
        if unmatched > 0 {
            return Err(ConvertError::Verify { unmatched });
        }
    } else {
        tracing::warn!(
            reference = %options.reference.display(),
            "stroke count reference missing, check skipped (generate it with mmh-reference)"
        );
    }
    if options.first_reference.exists() {
        let produced = read_firsts(table)?;
        let expected = read_first_reference(&options.first_reference)?;
        let known = read_whitelist(&options.first_whitelist)?;
        let outcome = compare_first(&chars, &produced, &expected, &known);
        tracing::info!(
            table = %table.display(),
            compared = outcome.compared,
            absent = outcome.absent,
            unreferenced = outcome.unreferenced,
            incomparable = outcome.incomparable,
            whitelisted = outcome.used.len(),
            unmatched = outcome.unmatched,
            "first stroke check done"
        );
        if outcome.used.len() < known.len() {
            tracing::warn!(
                unused = known.len() - outcome.used.len(),
                path = %options.first_whitelist.display(),
                "whitelisted characters matched this time and can be removed"
            );
        }
        if outcome.unmatched > 0 {
            return Err(ConvertError::Verify {
                unmatched: outcome.unmatched,
            });
        }
    } else {
        tracing::warn!(
            reference = %options.first_reference.display(),
            "first stroke reference missing, check skipped (generate it with mmh-reference)"
        );
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

/// 读产物首笔：字 → 键位类别（1→h、2→s、3→p、5→z、n→n）。
fn read_firsts(path: &Path) -> Result<HashMap<char, char>, ConvertError> {
    let mut firsts = HashMap::new();
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
        let class = match sequence.chars().next() {
            Some('1') => 'h',
            Some('2') => 's',
            Some('3') => 'p',
            Some('5') => 'z',
            Some('n') => 'n',
            _ => {
                return Err(malformed(
                    path,
                    number,
                    "stroke sequence starts with an unknown stroke",
                ));
            }
        };
        firsts.insert(ch, class);
    }
    Ok(firsts)
}

/// 读笔画数对照表：字 → 大陆笔画数。
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

/// 读首笔对照表：字 → 几何类别（h / s / p / n / z / ?）。
fn read_first_reference(path: &Path) -> Result<HashMap<char, char>, ConvertError> {
    let mut firsts = HashMap::new();
    for (index, line) in BufReader::new(File::open(path)?).lines().enumerate() {
        let line = line?;
        let number = index + 1;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(ch), Some(class)) = (fields.next(), fields.next()) else {
            return Err(malformed(
                path,
                number,
                "expected a character and a first-stroke class",
            ));
        };
        let Some(ch) = single_char(ch) else {
            return Err(malformed(path, number, "expected one character"));
        };
        if !matches!(class, "h" | "s" | "p" | "n" | "z" | "?") {
            return Err(malformed(
                path,
                number,
                "first-stroke class must be one of h s p n z ?",
            ));
        }
        firsts.insert(ch, class.chars().next().unwrap_or('?'));
    }
    Ok(firsts)
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
