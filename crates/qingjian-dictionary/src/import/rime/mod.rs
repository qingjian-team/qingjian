//! Rime 词库转换：单文件供形码工具使用，用户导入另合并分表并给无注音词补拼音。

mod entry;
mod header;
mod parsed;
mod readings;

#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::{DictionaryError, canonical_syllable};
use entry::Entry;
use header::Header;

pub use parsed::Parsed;

pub fn looks_like_rime(text: &str) -> bool {
    text.trim_start_matches('\u{feff}')
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .is_some_and(|l| l == "---" || l.starts_with("name:"))
}

/// 不跨文件、不自动注音，形码转换工具沿用这个入口。
pub fn to_tsv(text: &str) -> Parsed {
    let (header, entries) = parse(text);
    let mut tsv = String::with_capacity(text.len());
    for entry in entries.into_iter().filter(|entry| !entry.code.is_empty()) {
        entry.append_tsv(&mut tsv);
    }
    Parsed {
        tsv,
        name: header.name,
    }
}

/// 读主表及其引用，显式拼音全部收齐后才给无注音词补读音。
pub(super) fn read_all(source: &Path) -> Result<(Parsed, Header), DictionaryError> {
    let source = std::fs::canonicalize(source)?;
    let root = source.parent().unwrap_or(Path::new(".")).to_owned();
    let mut queue = vec![source];
    let mut visited = HashSet::new();
    let mut seen = HashSet::new();
    let mut entries = Vec::new();
    let mut pending = Vec::new();
    let mut main = None;
    while let Some(path) = queue.pop() {
        let identity = std::fs::canonicalize(&path)?;
        if !identity.starts_with(&root) {
            return Err(DictionaryError::Corrupt(
                "import_tables path escapes dictionary directory",
            ));
        }
        if !visited.insert(identity.clone()) {
            continue;
        }
        let (header, rows) = parse(&std::fs::read_to_string(&identity)?);
        for mut row in rows {
            if row.code.is_empty() {
                pending.push(row);
                continue;
            }
            row.code = row
                .code
                .split_whitespace()
                .map(canonical_syllable)
                .collect::<Vec<_>>()
                .join(" ");
            if seen.insert((row.word.clone(), row.code.clone())) {
                entries.push(row);
            }
        }
        // Rime 分表名相对所选主词库目录，嵌套分表也遵守这条规则。
        for name in header.imports.iter().rev() {
            queue.push(resolve_import(&root, name));
        }
        if main.is_none() {
            main = Some(header);
        }
    }
    let mut tsv = String::new();
    for entry in &entries {
        entry.append_tsv(&mut tsv);
    }
    let readings = readings::collect(&entries);
    let mut skipped = 0;
    for entry in pending {
        let Some(codes) = readings::encode(&entry.word, &readings) else {
            skipped += 1;
            continue;
        };
        for code in codes {
            if seen.insert((entry.word.clone(), code.clone())) {
                Entry {
                    word: entry.word.clone(),
                    code,
                    weight: entry.weight,
                }
                .append_tsv(&mut tsv);
            }
        }
    }
    if skipped > 0 {
        tracing::warn!(
            skipped,
            "Rime 词库部分词条缺少字表读音或读音组合超过 256，已跳过"
        );
    }
    let header = main.unwrap_or_default();
    Ok((
        Parsed {
            tsv,
            name: header.name.clone(),
        },
        header,
    ))
}

fn parse(text: &str) -> (Header, Vec<Entry>) {
    let text = text.trim_start_matches('\u{feff}');
    let (header, body) = Header::parse(text);
    let column = |name| header.columns.iter().position(|value| value == name);
    let text_column = column("text");
    let code_column = column("code");
    let weight_column = column("weight");
    let mut entries = Vec::new();
    for raw in text.lines().skip(body) {
        let line = raw.trim_end();
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('\t').map(str::trim).collect();
        let field = |index: Option<usize>| index.and_then(|i| fields.get(i)).copied().unwrap_or("");
        let word = field(text_column);
        if word.is_empty() {
            continue;
        }
        entries.push(Entry {
            word: word.to_owned(),
            code: field(code_column).to_owned(),
            // 保留旧导入器对非整数权重的处理；不引入 Rime 预设词频依赖。
            weight: field(weight_column).parse::<u32>().unwrap_or(1),
        });
    }
    (header, entries)
}

fn resolve_import(root: &Path, name: &str) -> PathBuf {
    let path = root.join(name);
    if path.is_file() || name.ends_with(".dict.yaml") {
        path
    } else {
        root.join(format!("{name}.dict.yaml"))
    }
}
