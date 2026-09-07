//! 英→中释义：给英文词表里的词写中文对应词，英文候选（中英混输、英文模式）右侧显示。
//! 词按 wordfreq 词频挑（`english.tsv` 第三列），技术词表里的词不看词频一律要。

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;

use crate::args::EnglishArgs;
use crate::batch::{self, Task};
use crate::client::LlmClient;
use crate::entry::EnglishGlossEntry;
use crate::error::GlossError;
use crate::prompt::normalize_pos;
use crate::store::Store;

/// 每个词最多留几个中文释义。
const MAX_SENSES: usize = 2;

/// 单个中文释义最多几个字：再长是解释不是对应词。
const MAX_SENSE_CHARS: usize = 8;

const SYSTEM_PROMPT: &str = "你是英汉词典编纂者。给每个英文词写最简短的中文对应词，供输入法在英文候选旁边一行显示，所以只要词、不要解释。\n\
规则：\n\
- pos：这个英文词最主要的词性，只能是 n. v. adj. adv. pron. prep. conj. num. int. phr. 之一（abbr. 缩写也写 n.，短语写 phr.）。\n\
- zh：1 到 2 个最贴切的中文对应词，按常用度排；每个不超过 6 个字；不要括号、不要解释、不要例句；多义词只取最常用的义项。\n\
- 专名照译（GitHub → GitHub，Python → Python）；技术词、缩写给业内通行的中文说法（kubectl → K8s 命令行、CDN → 内容分发网络）；\
没有把握也要给最可能的答案，不要留空。\n\
输出严格的 JSON：{\"items\":[{\"w\":\"develop\",\"pos\":\"v.\",\"zh\":[\"开发\",\"发展\"]}]}。\n\
items 与输入的词一一对应、顺序一致、每个词恰好一项，w 必须原样照抄输入的词。";

struct EnglishTask;

#[derive(Debug, Deserialize)]
struct RawReply {
    #[serde(default)]
    items: Vec<RawItem>,
}

#[derive(Debug, Deserialize)]
struct RawItem {
    w: String,

    #[serde(default)]
    pos: Option<String>,

    #[serde(default)]
    zh: Vec<String>,
}

impl Task for EnglishTask {
    type Entry = EnglishGlossEntry;

    fn system_prompt(&self) -> &str {
        SYSTEM_PROMPT
    }

    fn user_prompt(&self, words: &[String]) -> String {
        let mut text = String::from("词：\n");
        for word in words {
            text.push_str(word);
            text.push('\n');
        }
        text
    }

    fn parse_reply(
        &self,
        content: &str,
        words: &[String],
    ) -> Result<Vec<EnglishGlossEntry>, serde_json::Error> {
        let reply: RawReply = serde_json::from_str(content.trim())?;
        let mut entries: Vec<EnglishGlossEntry> = Vec::with_capacity(words.len());
        for item in reply.items {
            let word = item.w.trim();
            if !words.iter().any(|w| w == word) || entries.iter().any(|e| e.word == word) {
                continue;
            }
            let zh = clean_chinese(item.zh);
            if zh.is_empty() {
                continue;
            }
            entries.push(EnglishGlossEntry {
                word: word.to_owned(),
                pos: item.pos.as_deref().and_then(normalize_pos),
                zh,
            });
        }
        Ok(entries)
    }
}

/// 去空、去尾标点、限长、去重，最多两个。
fn clean_chinese(raw: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for text in raw {
        let text = text
            .trim()
            .trim_end_matches(['。', '；', '，', '.', ';', ','])
            .trim();
        let ok = !text.is_empty()
            && text.chars().count() <= MAX_SENSE_CHARS
            && !text.contains(['(', '（', '：', ':']);
        if ok && !out.iter().any(|o| o == text) {
            out.push(text.to_owned());
        }
        if out.len() == MAX_SENSES {
            break;
        }
    }
    out
}

/// 挑词：`english.tsv`（`词\t编码\t词频`）里词频不低于 `min_frequency` 的，加上 `include` 各表（带表头的 TSV，第一列）里的全部词；
/// 按词频降序，最多 `limit` 个。
fn select(
    words: &Path,
    min_frequency: u32,
    include: &[PathBuf],
    limit: Option<usize>,
) -> Result<Vec<String>, GlossError> {
    let mut always: HashSet<String> = HashSet::new();
    for path in include {
        for line in std::fs::read_to_string(path)?.lines().skip(1) {
            if let Some(word) = line.split('\t').next()
                && !word.trim().is_empty()
            {
                always.insert(word.trim().to_owned());
            }
        }
    }
    let mut rows: Vec<(String, u32)> = std::fs::read_to_string(words)?
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let mut fields = l.split('\t');
            let word = fields.next()?.trim();
            let frequency: u32 = fields
                .nth(1)
                .and_then(|f| f.trim().parse().ok())
                .unwrap_or(0);
            (frequency >= min_frequency || always.contains(word))
                .then(|| (word.to_owned(), frequency))
        })
        .collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    rows.dedup_by(|a, b| a.0 == b.0);
    if let Some(limit) = limit {
        rows.truncate(limit);
    }
    if rows.is_empty() {
        return Err(GlossError::NoWords(words.to_owned()));
    }
    Ok(rows.into_iter().map(|(w, _)| w).collect())
}

pub async fn run(args: EnglishArgs) -> Result<(), GlossError> {
    let selected = select(&args.words, args.min_frequency, &args.include, args.limit)?;
    let store = Arc::new(Store::<EnglishGlossEntry>::open(&args.out)?);
    let pending: Vec<String> = selected
        .iter()
        .filter(|w| !store.contains(w))
        .cloned()
        .collect();
    tracing::info!(
        selected = selected.len(),
        already_done = store.len(),
        pending = pending.len(),
        model = %args.model,
        "开始生成英→中释义"
    );
    if pending.is_empty() {
        return Ok(());
    }
    let client = Arc::new(LlmClient::new(
        &args.base_url,
        &args.api_key,
        &args.model,
        Duration::from_secs(args.timeout_secs),
    ));
    batch::run(
        Arc::new(EnglishTask),
        client,
        store,
        pending,
        args.batch,
        args.concurrency,
    )
    .await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_and_limits_chinese_senses() {
        let cleaned = clean_chinese(vec![
            " 开发。".to_owned(),
            "开发".to_owned(),
            "发展（经济）".to_owned(),
            "研制".to_owned(),
            "第三个".to_owned(),
        ]);
        assert_eq!(cleaned, ["开发", "研制"]);
    }

    #[test]
    fn parses_only_requested_words() {
        let words = vec!["develop".to_owned()];
        let reply = r#"{"items":[{"w":"develop","pos":"v.","zh":["开发","发展"]},{"w":"other","pos":"n.","zh":["其他"]}]}"#;
        let entries = EnglishTask.parse_reply(reply, &words).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].pos.as_deref(), Some("v."));
        assert_eq!(entries[0].zh, ["开发", "发展"]);
    }
}
