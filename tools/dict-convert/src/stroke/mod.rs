//! 笔画码表：CNS11643 全字庫「筆順資料」+ 大陆序覆盖表 → `codes/stroke.tsv`（随包笔画表的源数据）。
//!
//! 产物每行 `字\t序列`：序列按大陆笔顺，1 横 / 2 竖 / 3 撇 / 5 折 / n 点与捺
//! （CNS 的 4 同时涵盖点与捺，按「点捺合并为 n」写 n；提在 CNS 里多数已记 1，氵的第三笔记 4，由覆盖表改回 1）。
//! 数据链：
//!
//! 1. `--cns-map`：官方 CNS→Unicode 对照表（`CNS2UNICODE_Unicode*.txt`，UTF-8、行内容纯 ASCII），建 CNS 字碼 → 字；
//! 2. `--cns-seq`：官方筆順資料（`CNS_strokes_sequence.txt`），`CNS 字碼\t[1-5]{n}`，经上一步映射成 字 → 序列
//!    （同一个字落到两条不同序列上直接报错：那说明对照表或笔顺表读错了）；
//! 3. `--cns-count`：官方筆畫數（`CNS_stroke.txt`，可选），序列长度与它自洽（差不超过 `--max-diff`）的字才留——
//!    CNS 的序列与筆畫數在罕用字上互相打架（t07 实测 |d| > 1 约 0.5%），**以序列为准、以筆畫數过滤**；
//! 4. `--prc-rules`：大陆序覆盖表（`assets/stroke/prc-rules.tsv`），台湾序按部件重写成大陆序；
//! 5. `--filter`：字表白名单（缺省通用规范字表 8,105 字），只出表里的字，按表序排列。
//!
//! `--verify` 再按抽样对照表逐字比对大陆笔画数：不符的字必须都在白名单里，否则退出码非 0。
//! 数据来源、许可与验收记录见 `assets/stroke/README.md`；随包前由 `pack codes`（见 `codes` 模块）按取码规则把它与词库算成码表。

mod options;
mod rules;
mod verify;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::error::ConvertError;
use crate::stroke::rules::PrcRules;
pub(crate) use crate::stroke::rules::{malformed, single_char};

pub use crate::stroke::options::StrokeOptions;

/// 对照表文件名的前缀（给目录用）。
const CNS_MAP_PREFIX: &str = "CNS2UNICODE";

/// CNS 序列里出现的笔画码。
const STROKE_DIGITS: &str = "12345";

/// 生成笔画表。
pub fn convert(options: &StrokeOptions, out_dir: &Path) -> Result<(), ConvertError> {
    let started = Instant::now();
    let rules = PrcRules::from_path(&options.prc_rules)?;
    tracing::info!(
        rules = rules.rule_count(),
        overrides = rules.override_count(),
        path = %options.prc_rules.display(),
        "已读大陆序覆盖表"
    );
    let chars = read_cns_map(&options.cns_map)?;
    tracing::info!(entries = chars.len(), "已读 CNS→Unicode 对照表");
    let sequences = read_sequences(&options.cns_seq, &chars)?;
    tracing::info!(entries = sequences.len(), "已读筆順資料");
    let stroke_counts = match options.cns_count.as_deref() {
        Some(path) => {
            let counts = read_stroke_counts(path, &chars)?;
            tracing::info!(entries = counts.len(), "已读筆畫數");
            counts
        }
        None => HashMap::new(),
    };
    let table = read_filter(&options.filter)?;
    tracing::info!(chars = table.len(), path = %options.filter.display(), "已读字表白名单");

    let out = options
        .output
        .clone()
        .unwrap_or_else(|| out_dir.join("codes").join("stroke.tsv"));
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut writer = BufWriter::new(File::create(&out)?);
    let mut written = 0usize;
    let mut missing = 0usize;
    let mut inconsistent = 0usize;
    let mut digits = 0usize;
    for ch in &table {
        let original = sequences.get(ch);
        if let (Some(sequence), Some(count)) = (original, stroke_counts.get(ch))
            && sequence.len().abs_diff(*count as usize) > options.max_diff
        {
            inconsistent += 1;
            continue;
        }
        let sequence = match original {
            Some(sequence) => rules.apply(*ch, sequence),
            None => match rules.override_sequence(*ch) {
                Some(sequence) => sequence.to_owned(),
                None => {
                    missing += 1;
                    continue;
                }
            },
        };
        let sequence = normalize(&sequence);
        writeln!(writer, "{ch}\t{sequence}")?;
        written += 1;
        digits += sequence.len();
    }
    writer.flush()?;
    let average = digits as f64 / written.max(1) as f64;

    let size = std::fs::metadata(&out).map(|meta| meta.len()).unwrap_or(0);
    tracing::info!(
        out = %out.display(),
        entries = written,
        dropped_no_sequence = missing,
        dropped_inconsistent = inconsistent,
        average_strokes = average,
        size_kb = size / 1_000,
        elapsed_ms = started.elapsed().as_millis(),
        "已生成笔画表"
    );
    if options.verify {
        verify::run(
            &out,
            &options.sample,
            options.stride,
            &options.reference,
            &options.whitelist,
        )?;
    }
    Ok(())
}

/// 点与捺合并成 `n`：产物只用 1 2 3 5 n 五个符号，与大陆厂商的 h/s/p/n/z 键位对得上。
fn normalize(sequence: &str) -> String {
    sequence
        .chars()
        .map(|c| if c == '4' { 'n' } else { c })
        .collect()
}

/// 读 CNS→Unicode 对照表：可以给文件，也可以给目录（目录取其中的对照表）。
fn read_cns_map(paths: &[PathBuf]) -> Result<HashMap<String, char>, ConvertError> {
    let mut map = HashMap::new();
    for path in expand_map_paths(paths)? {
        for (index, line) in lines_of(&path)?.enumerate() {
            let line = line?;
            let number = index + 1;
            if line.is_empty() {
                continue;
            }
            let mut fields = line.split('\t');
            let (Some(code), Some(value)) = (fields.next(), fields.next()) else {
                return Err(malformed(
                    &path,
                    number,
                    "expected a CNS code and a code point",
                ));
            };
            let point = u32::from_str_radix(value, 16)
                .map_err(|_| malformed(&path, number, "code point is not hexadecimal"))?;
            let Some(ch) = char::from_u32(point) else {
                return Err(malformed(&path, number, "code point is not a character"));
            };
            map.entry(code.to_owned()).or_insert(ch);
        }
    }
    Ok(map)
}

/// 把 `--cns-map` 给的路径展开成文件列表：目录取其中以 `CNS2UNICODE` 开头的 `.txt`，按名字排序。
fn expand_map_paths(paths: &[PathBuf]) -> Result<Vec<PathBuf>, ConvertError> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            let mut found: Vec<PathBuf> = std::fs::read_dir(path)?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|entry| {
                    entry.extension().is_some_and(|ext| ext == "txt")
                        && entry
                            .file_name()
                            .is_some_and(|name| name.to_string_lossy().starts_with(CNS_MAP_PREFIX))
                })
                .collect();
            found.sort();
            files.extend(found);
        } else {
            files.push(path.clone());
        }
    }
    if files.is_empty() {
        return Err(ConvertError::Format {
            path: paths.first().cloned().unwrap_or_default(),
            line: 0,
            reason: "no CNS to Unicode table found".to_owned(),
        });
    }
    Ok(files)
}

/// 读筆順資料，经对照表映射成 字 → 序列。
fn read_sequences(
    path: &Path,
    chars: &HashMap<String, char>,
) -> Result<HashMap<char, String>, ConvertError> {
    let mut sequences: HashMap<char, String> = HashMap::new();
    for (index, line) in lines_of(path)?.enumerate() {
        let line = line?;
        let number = index + 1;
        if line.is_empty() {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(code), Some(sequence)) = (fields.next(), fields.next()) else {
            return Err(malformed(
                path,
                number,
                "expected a CNS code and a stroke sequence",
            ));
        };
        if sequence.is_empty() || !sequence.chars().all(|c| STROKE_DIGITS.contains(c)) {
            return Err(malformed(
                path,
                number,
                "stroke sequence must be digits 1-5",
            ));
        }
        let Some(ch) = chars.get(code) else {
            continue;
        };
        match sequences.get(ch) {
            Some(known) if known != sequence => {
                return Err(malformed(
                    path,
                    number,
                    "two stroke sequences for one character",
                ));
            }
            Some(_) => {}
            None => {
                sequences.insert(*ch, sequence.to_owned());
            }
        }
    }
    Ok(sequences)
}

/// 读筆畫數（第二个字段是 10 进制的笔画数）。
fn read_stroke_counts(
    path: &Path,
    chars: &HashMap<String, char>,
) -> Result<HashMap<char, u32>, ConvertError> {
    let mut counts = HashMap::new();
    for (index, line) in lines_of(path)?.enumerate() {
        let line = line?;
        let number = index + 1;
        if line.is_empty() {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(code), Some(value)) = (fields.next(), fields.next()) else {
            return Err(malformed(
                path,
                number,
                "expected a CNS code and a stroke count",
            ));
        };
        let count = value
            .parse::<u32>()
            .map_err(|_| malformed(path, number, "stroke count is not a number"))?;
        if let Some(ch) = chars.get(code) {
            counts.entry(*ch).or_insert(count);
        }
    }
    Ok(counts)
}

/// 读字表白名单：一行一个字（第一列），别的列忽略；表序即输出序。
fn read_filter(path: &Path) -> Result<Vec<char>, ConvertError> {
    let mut chars = Vec::new();
    let mut seen = HashSet::new();
    for line in lines_of(path)? {
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

/// 按行读一个文件。
fn lines_of(path: &Path) -> Result<std::io::Lines<BufReader<File>>, ConvertError> {
    Ok(BufReader::new(File::open(path)?).lines())
}
