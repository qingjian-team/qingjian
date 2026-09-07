//! 英文词表：每行 `词\t编码[\t…]`（表头行、YAML 头这类没有制表符或编码不是字母的行自然跳过），`#` 为注释。只收纯字母的词。

use std::collections::{HashMap, HashSet};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::error::ConvertError;

/// `frequency` 是可选的 `编码\t词频` 表，有就写进第三列。
pub fn convert(
    inputs: &[PathBuf],
    frequency: Option<&Path>,
    output: &Path,
) -> Result<(), ConvertError> {
    let frequencies: HashMap<String, u32> = match frequency {
        Some(path) => std::fs::read_to_string(path)?
            .lines()
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .filter_map(|l| {
                let (code, count) = l.split_once('\t')?;
                Some((code.to_owned(), count.trim().parse().ok()?))
            })
            .collect(),
        None => HashMap::new(),
    };
    let mut seen: HashSet<String> = HashSet::new();
    let mut rows: Vec<(String, String)> = Vec::new();
    for path in inputs {
        let before = rows.len();
        for raw in std::fs::read_to_string(path)?.lines() {
            let line = raw.trim_end();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let word = fields.next().unwrap_or_default().trim();
            let Some(code) = fields.next().map(str::trim) else {
                continue;
            };
            let code = code.to_ascii_lowercase();
            if word.is_empty()
                || !word.bytes().all(|b| b.is_ascii_alphabetic())
                || !seen.insert(code.clone())
            {
                continue;
            }
            rows.push((word.to_owned(), code));
        }
        tracing::info!(path = %path.display(), entries = rows.len() - before, "已读取");
    }
    let mut file = BufWriter::new(std::fs::File::create(output)?);
    writeln!(
        file,
        "# 由 qingjian-dict-convert english 生成。词\\t编码\\t词频（wordfreq 的 Zipf 频率 ×1000）"
    )?;
    for (word, code) in &rows {
        let count = frequencies.get(code).copied().unwrap_or(0);
        writeln!(file, "{word}\t{code}\t{count}")?;
    }
    file.flush()?;
    tracing::info!(path = %output.display(), entries = rows.len(), "写出完成");
    Ok(())
}
