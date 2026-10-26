//! 冷启动字词评测：独立内存学习器、不上屏、不写个人数据，原候选与生成结果逐条保留。

mod entry;
pub mod error;

use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;

use qingjian_core::Engine;
use qingjian_neural::{CharScorer, P2c};
use serde_json::json;

use entry::Entry;
use error::ColdError;

pub fn run(
    engine: &mut Engine,
    input: &Path,
    output: &Path,
    model: Option<&Path>,
) -> Result<(), ColdError> {
    let entries: Vec<Entry> = std::fs::read_to_string(input)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    let mut ids = HashSet::new();
    for row in &entries {
        if row.text.is_empty()
            || row.keys.is_empty()
            || !row
                .keys
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '\'')
            || !ids.insert(&row.id)
        {
            return Err(ColdError::Invalid(row.id.clone()));
        }
    }
    let dictionaries = std::iter::once(engine.dictionary()).chain(engine.extra_dictionaries());
    let mut readings: HashMap<String, HashSet<String>> = HashMap::new();
    for dictionary in dictionaries {
        for word in dictionary.entries() {
            readings
                .entry(word.text.to_owned())
                .or_default()
                .insert(word.pinyin.replace(' ', ""));
        }
    }
    let scorer = model.map(CharScorer::load).transpose()?;
    let decoder = scorer
        .as_ref()
        .map(|s| P2c::new(s.model(), s.vocab()).ok_or(ColdError::Model))
        .transpose()?;
    let mut output = BufWriter::new(
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)?,
    );
    for (index, entry) in entries.iter().enumerate() {
        engine.clear();
        engine.break_chain();
        engine.history_mut().clear();
        engine.set_rescoring_context(None);
        engine.set_input(&entry.keys);
        let start = Instant::now();
        let query = engine.query();
        let engine_ms = start.elapsed().as_secs_f64() * 1000.0;
        let (baseline, candidates, query_error) = match query {
            Ok(query) => (
                query
                    .candidates
                    .items
                    .iter()
                    .map(|c| c.text.clone())
                    .collect::<Vec<_>>(),
                serde_json::to_value(&query.candidates.items)?,
                None,
            ),
            Err(error) => (Vec::new(), json!([]), Some(error.to_string())),
        };
        let keys = entry.keys.replace('\'', "");
        let generated_start = Instant::now();
        // 固定 beam 与生成上限，不能用目标字数限制搜索。分隔符只从 P2C 输入去掉。
        let generated = decoder
            .as_ref()
            .map(|d| d.convert(&keys, 5, 16))
            .transpose()?
            .unwrap_or_default();
        let generation_ms = generated_start.elapsed().as_secs_f64() * 1000.0;
        let p2c: Vec<String> = generated
            .iter()
            .filter(|c| !c.text.is_empty() && c.text.chars().all(is_han))
            .map(|c| c.text.clone())
            .collect();
        let merged = merge(&baseline, &p2c);
        let present = readings.get(&entry.text);
        let row = json!({
            "entry": entry, "dictionary_text": present.is_some(),
            "dictionary_reading": present.is_some_and(|p| p.contains(&keys)),
            "baseline": baseline, "candidate_metadata": candidates,
            "p2c": p2c, "generated_raw": generated.iter().map(|c| json!({"text":c.text,"score":c.score})).collect::<Vec<_>>(),
            "merged": merged, "query_error": query_error,
            "engine_ms": engine_ms, "generation_ms": generation_ms,
        });
        writeln!(output, "{row}")?;
        if (index + 1) % 250 == 0 {
            output.flush()?;
            tracing::info!(
                done = index + 1,
                total = entries.len(),
                "冷启动字词评测进度"
            );
        }
    }
    output.flush()?;
    println!(
        "冷启动字词评测完成：{} 条；个人数据未加载，未上屏学习",
        entries.len()
    );
    Ok(())
}

/// 可复现的实验合并策略：原首选保留，生成中新增的词放其后，再接原候选。不是最终产品排序。
fn merge(baseline: &[String], generated: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for text in baseline
        .iter()
        .take(1)
        .chain(generated.iter().filter(|s| !baseline.contains(s)))
        .chain(baseline.iter().skip(1))
    {
        if !out.contains(text) {
            out.push(text.clone());
        }
    }
    out
}

fn is_han(c: char) -> bool {
    matches!(c, '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}')
}

#[cfg(test)]
mod tests {
    use super::merge;
    use crate::args::Args;
    use clap::Parser;

    #[test]
    fn isolation_rejects_personal_files_and_config() {
        for flag in ["--user-dict", "--config"] {
            assert!(
                Args::try_parse_from([
                    "cli",
                    "--eval-cold",
                    "in.jsonl",
                    "--cold-output",
                    "out.jsonl",
                    flag,
                    "personal.tsv"
                ])
                .is_err()
            );
        }
    }

    #[test]
    fn merging_preserves_baseline_and_adds_only_novel_terms() {
        let old = ["意识", "一时", "仪式"].map(String::from);
        let new = ["一时", "一十", "医师"].map(String::from);
        assert_eq!(merge(&old, &new), ["意识", "一十", "医师", "一时", "仪式"]);
    }
}
