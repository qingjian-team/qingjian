//! 开发期对照数据：从 hanzi-writer-data（Make Me a Hanzi，大陆规范字形）生成 `stroke --verify`
//! 用的两张对照表。
//!
//! 数据许可是 Arphic Public License，与仓库的 GPL-3.0 不兼容：只作开发期对照，**不进仓库、不随包**，
//! 产物写在 `data/mmh/`（gitignore），`--verify` 找不到哪张就跳过哪张对照。两个产物：
//!
//! - `prc-counts-l1.tsv`：`字<TAB>`大陆笔画数（每字 medians 的笔画条数）；
//! - `prc-first-strokes-l1.tsv`：`字<TAB>`首笔几何类别，类别见 `first_class`。
//!
//! 首笔类别是首笔中点折线的几何近似，只用来**找**不一致：带尖角转折的记 z、竖撇的走向记 s 这类，
//! 都以规范裁定量为准，不是规范口径本身；规范依据与白名单见 `assets/stroke/README.md`。

mod options;

#[cfg(test)]
mod tests;

pub use crate::mmh::options::MmhReferenceOptions;

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::error::ConvertError;
use crate::stroke::{malformed, single_char};

/// 笔画数对照的文件名。
const COUNTS_FILE: &str = "prc-counts-l1.tsv";

/// 首笔对照的文件名。
const FIRSTS_FILE: &str = "prc-first-strokes-l1.tsv";

/// 生成两张对照表。
pub fn generate(options: &MmhReferenceOptions) -> Result<(), ConvertError> {
    let chars = read_filter(&options.filter)?;
    tracing::info!(chars = chars.len(), path = %options.filter.display(), "已读字表");
    std::fs::create_dir_all(&options.out_dir)?;
    let counts_path = options.out_dir.join(COUNTS_FILE);
    let firsts_path = options.out_dir.join(FIRSTS_FILE);
    let mut counts = BufWriter::new(File::create(&counts_path)?);
    let mut firsts = BufWriter::new(File::create(&firsts_path)?);
    writeln!(
        counts,
        "# 一级字大陆笔画数对照表（开发期对照用；字<TAB>笔画数）"
    )?;
    writeln!(
        counts,
        "# 来源：hanzi-writer-data（Make Me a Hanzi，PRC 字形；Arphic Public License）每字 medians 条数。"
    )?;
    writeln!(
        counts,
        "# 不进仓库、不随包分发；再生成：cargo run -p qingjian-dict-convert -- mmh-reference（见 assets/stroke/README.md）。"
    )?;
    writeln!(
        firsts,
        "# 一级字大陆首笔几何对照表（开发期对照用；字<TAB>类别 h/s/p/n/z/?）"
    )?;
    writeln!(
        firsts,
        "# 来源：hanzi-writer-data（Make Me a Hanzi；Arphic Public License）首笔 medians 的几何分类。"
    )?;
    writeln!(
        firsts,
        "# 类别只用来找不一致，不是规范口径；再生成：cargo run -p qingjian-dict-convert -- mmh-reference。"
    )?;
    let mut written = 0usize;
    let mut missing = 0usize;
    for ch in &chars {
        let Some(medians) = read_char(&options.mmh, *ch)? else {
            missing += 1;
            continue;
        };
        let strokes = medians.len();
        let first = medians.first().map_or('?', |median| first_class(median));
        writeln!(counts, "{ch}\t{strokes}")?;
        writeln!(firsts, "{ch}\t{first}")?;
        written += 1;
    }
    counts.flush()?;
    firsts.flush()?;
    tracing::info!(
        counts = %counts_path.display(),
        firsts = %firsts_path.display(),
        written,
        missing,
        "已生成对照表"
    );
    Ok(())
}

/// 一个字的中点折线：每笔一条折线，点对是 1024 见方里的 x / y（y 向上）。
type Medians = Vec<Vec<[f64; 2]>>;

/// 读一个字的字形数据；文件不存在返回 None（对照源不必覆盖全字表）。
fn read_char(dir: &Path, ch: char) -> Result<Option<Medians>, ConvertError> {
    let path = dir.join(format!("{ch}.json"));
    if !path.is_file() {
        return Ok(None);
    }
    let value: serde_json::Value = serde_json::from_reader(File::open(&path)?)?;
    let Some(medians) = value.get("medians").and_then(serde_json::Value::as_array) else {
        return Err(malformed(&path, 0, "expected a medians array"));
    };
    let mut parsed: Medians = Vec::with_capacity(medians.len());
    for (index, median) in medians.iter().enumerate() {
        let line = index + 1;
        let points = median
            .as_array()
            .ok_or_else(|| malformed(&path, line, "expected a median point array"))?;
        let mut stroke = Vec::with_capacity(points.len());
        for point in points {
            stroke.push(read_point(&path, line, point)?);
        }
        parsed.push(stroke);
    }
    Ok(Some(parsed))
}

/// 读折线上的一对坐标。
fn read_point(
    path: &Path,
    line: usize,
    point: &serde_json::Value,
) -> Result<[f64; 2], ConvertError> {
    let pair = point
        .as_array()
        .ok_or_else(|| malformed(path, line, "expected a point pair"))?;
    if pair.len() != 2 {
        return Err(malformed(path, line, "expected an x/y pair"));
    }
    let x = pair[0]
        .as_f64()
        .ok_or_else(|| malformed(path, line, "x is not a number"))?;
    let y = pair[1]
        .as_f64()
        .ok_or_else(|| malformed(path, line, "y is not a number"))?;
    Ok([x, y])
}

/// 首笔的几何类别：首笔中点折线（1024 见方，y 向上）分类成 h / s / p / n / z / ?。
///
/// 只为「找不一致」服务：前半程与后半程的平均方向夹角超过 55° 的是折（横折、竖提、撇点这类
/// 一处大拐弯记 z；撇是渐弯的，前后半程方向差到不了 55°，不会误成 z）。没有转折的按走向分：
/// 向右或右上记 h（横、提），向左下记 p（撇，含浅的平撇），近竖直向下记 s（竖撇的走向也在这里，
/// 对照时按近似对接受），向右下记 n（捺）；太短分不出走向的记 n（点），退化到没有方向的记 ?。
pub(super) fn first_class(median: &[[f64; 2]]) -> char {
    if median.len() < 2 {
        return '?';
    }
    let scale = 1_024.0;
    if half_angle(median, scale) > 55.0 {
        return 'z';
    }
    let length: f64 = median
        .windows(2)
        .map(|pair| (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1]) / scale)
        .sum();
    if length < 0.02 {
        return 'n';
    }
    let first = median[0];
    let last = median[median.len() - 1];
    let dx = (last[0] - first[0]) / scale;
    let dy = (last[1] - first[1]) / scale;
    if dy >= 0.0 {
        return if dx > 0.0 { 'h' } else { '?' };
    }
    if dy.abs() > 2.0 * dx.abs() {
        return 's';
    }
    if dx < 0.0 {
        return 'p';
    }
    'n'
}

/// 前半程与后半程的平均方向夹角（度）。按路径长度取中点，退化时给 0（没有转折证据）。
fn half_angle(median: &[[f64; 2]], scale: f64) -> f64 {
    if median.len() < 3 {
        return 0.0;
    }
    let total: f64 = median
        .windows(2)
        .map(|pair| (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1]) / scale)
        .sum();
    if total <= 0.0 {
        return 0.0;
    }
    // 中点落在第 split 段之后：前半程到 median[split] 为止；夹在首尾之间，两半都有内容
    let mut acc = 0.0;
    let mut split = median.len() - 2;
    for (index, pair) in median.windows(2).enumerate() {
        acc += (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1]) / scale;
        if acc >= total / 2.0 {
            split = (index + 1).min(median.len() - 2);
            break;
        }
    }
    let direction =
        |from: &[f64; 2], to: &[f64; 2]| [(to[0] - from[0]) / scale, (to[1] - from[1]) / scale];
    let first = direction(&median[0], &median[split]);
    let second = direction(&median[split], &median[median.len() - 1]);
    let norm = first[0].hypot(first[1]) * second[0].hypot(second[1]);
    if norm <= 0.0 {
        return 0.0;
    }
    let dot = (first[0] * second[0] + first[1] * second[1]) / norm;
    dot.clamp(-1.0, 1.0).acos().to_degrees()
}

/// 读字表：一行一个字（第一列），表序即输出序。
fn read_filter(path: &Path) -> Result<Vec<char>, ConvertError> {
    let mut chars = Vec::new();
    let mut seen = HashSet::new();
    for line in std::fs::read_to_string(path)?.lines() {
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
