//! 导入码表：把用户给的 Rime `.dict.yaml`（含 `import_tables` 合表）或现成的 `.qj` 变成码表目录里的一个 `.qj`。
//!
//! 与词库导入的差别：码表不能静默丢数据。有词没有码的行、码不是 `a-z` 的行都进统计返回给调用方，
//! 一个可用条目都没有时直接报错（不是码表文件，或码列全是垃圾）。

use std::path::{Path, PathBuf};

use qingjian_format::{Container, Metadata};

use super::imported::AuxCodeTableImport;
use super::parsed::{ParsedTable, stem_of};
use super::report::AuxCodeTableImportReport;
use super::table::AuxCodeTable;
use crate::error::DictionaryError;

/// `import_tables` 最多跟几层：互相引用时绕不出来，到这就停。
const MAX_IMPORT_DEPTH: usize = 8;

/// 把 `source` 导入到 `dest_dir`，返回写出的文件、码表名与统计。
pub fn import_aux_code_table(
    source: &Path,
    dest_dir: &Path,
) -> Result<AuxCodeTableImport, DictionaryError> {
    let stem = stem_of(source).ok_or(DictionaryError::Corrupt("source has no usable file name"))?;
    std::fs::create_dir_all(dest_dir)?;
    let target = dest_dir.join(format!("{stem}.qj"));
    let (table, mut metadata, report) = if Container::is_qj(source) {
        let table = AuxCodeTable::open(source)?;
        let metadata = table.metadata().cloned().unwrap_or_default();
        let report = AuxCodeTableImportReport {
            read: table.len(),
            with_code: table.len(),
            ..AuxCodeTableImportReport::default()
        };
        (table, metadata, report)
    } else {
        let (parsed, report) = read_all(source)?;
        if parsed.pairs.is_empty() {
            return Err(DictionaryError::NoCodeEntries);
        }
        let table = AuxCodeTable::from_pairs(parsed.pairs)?;
        let metadata = Metadata {
            name: parsed.name.unwrap_or_default(),
            version: parsed.version.unwrap_or_default(),
            source: source.display().to_string(),
            ..Metadata::default()
        };
        (table, metadata, report)
    };
    if metadata.name.is_empty() {
        metadata.name = stem;
    }
    table.write_qj(&target, &metadata)?;
    Ok(AuxCodeTableImport {
        path: target,
        name: metadata.name,
        report,
    })
}

/// 读主文件与它 `import_tables` 指到的其他码表，合并成一份解析结果与一份统计。
fn read_all(source: &Path) -> Result<(ParsedTable, AuxCodeTableImportReport), DictionaryError> {
    let mut merged = ParsedTable::default();
    let mut report = AuxCodeTableImportReport::default();
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut queue: Vec<(PathBuf, usize)> = vec![(source.to_owned(), 0)];
    let mut main = true;
    while let Some((path, depth)) = queue.pop() {
        if depth > MAX_IMPORT_DEPTH {
            tracing::warn!(file = %path.display(), "import_tables 嵌套太深，跳过");
            continue;
        }
        let identity = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if seen.contains(&identity) {
            continue;
        }
        seen.push(identity);
        let parsed = ParsedTable::parse(&std::fs::read_to_string(&path)?);
        report.read += parsed.read;
        report.with_code += parsed.with_code;
        report.no_code += parsed.no_code;
        report.skipped += parsed.skipped;
        if main {
            merged.name = parsed.name.clone();
            merged.version = parsed.version.clone();
            main = false;
        }
        merged.pairs.extend(parsed.pairs);
        let dir = path.parent().unwrap_or(Path::new(".")).to_owned();
        for name in parsed.import_tables.iter().rev() {
            queue.push((resolve_import(&dir, name), depth + 1));
        }
    }
    Ok((merged, report))
}

/// `import_tables` 里的一条解析成路径：原样存在的先用，否则按 Rime 的惯例补 `.dict.yaml`。
fn resolve_import(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if candidate.exists() {
        return candidate;
    }
    for suffix in [".dict.yaml", ".yaml", ".yml"] {
        let candidate = dir.join(format!("{name}{suffix}"));
        if candidate.exists() {
            return candidate;
        }
    }
    candidate
}
