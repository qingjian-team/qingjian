//! 拼音标注：给多音字词标读音。词表一行一个词（`dict-convert lexicon --emit-ambiguous` 写出的），
//! 结果 JSONL 每行 `{"word":"重庆","pinyin":["chong","qing"]}`，`dict-convert lexicon --pinyin` 读回去、再按 Unihan 校验。

use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;

use crate::args::PinyinArgs;
use crate::batch::{self, Task};
use crate::client::LlmClient;
use crate::entry::PinyinEntry;
use crate::error::GlossError;
use crate::store::Store;

const SYSTEM_PROMPT: &str = "你是普通话审音专家。给每个中文词标注拼音，供拼音输入法建词库用。\n\
规则：\n\
- 多音字按这个词里的实际读音（重庆 chong2 qing4，长沙 chang2 sha1，蚌埠 beng4 bu4，行长 hang2 zhang3，六安 lu4 an1）；地名、人名、专业术语按通行读法。\n\
- 每个汉字恰好一个音节，音节之间空格，带声调数字 1–4，轻声 5；ü 写 v（女 nv3，略 lve4）；儿化的「儿」单独一个音节 er。\n\
- 不要解释，没有把握也要给最可能的读音。\n\
输出严格的 JSON：{\"items\":[{\"w\":\"重庆\",\"py\":\"chong2 qing4\"}]}。items 与输入的词一一对应、顺序一致、每个词恰好一项，w 必须原样照抄输入的词。";

/// 拼音任务。
struct PinyinTask;

#[derive(Debug, Deserialize)]
struct RawReply {
    #[serde(default)]
    items: Vec<RawItem>,
}

#[derive(Debug, Deserialize)]
struct RawItem {
    w: String,

    #[serde(default)]
    py: String,
}

impl Task for PinyinTask {
    type Entry = PinyinEntry;

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

    /// 只收请求过的词；音节数必须等于字数，音节只能是字母（声调数字去掉）。
    fn parse_reply(
        &self,
        content: &str,
        words: &[String],
    ) -> Result<Vec<PinyinEntry>, serde_json::Error> {
        let reply: RawReply = serde_json::from_str(content.trim())?;
        let mut entries: Vec<PinyinEntry> = Vec::with_capacity(words.len());
        for item in reply.items {
            let word = item.w.trim();
            if !words.iter().any(|w| w == word) || entries.iter().any(|e| e.word == word) {
                continue;
            }
            let Some(pinyin) = clean_pinyin(&item.py, word.chars().count()) else {
                tracing::debug!(word, py = %item.py, "拼音格式不对，丢弃");
                continue;
            };
            entries.push(PinyinEntry {
                word: word.to_owned(),
                pinyin,
            });
        }
        Ok(entries)
    }
}

/// `chong2 qing4` → `["chong", "qing"]`；音节数不等于字数、含非字母字符（去掉声调数字与 ü 转 v 之后）就不要。
fn clean_pinyin(raw: &str, chars: usize) -> Option<Vec<String>> {
    let syllables: Vec<String> = raw
        .split(|c: char| c.is_whitespace() || c == '\'')
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.chars()
                .filter(|c| !c.is_ascii_digit())
                .map(|c| match c {
                    'ü' | 'Ü' => 'v',
                    c => c.to_ascii_lowercase(),
                })
                .collect::<String>()
        })
        .collect();
    let ok = syllables.len() == chars
        && syllables
            .iter()
            .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_lowercase()));
    ok.then_some(syllables)
}

pub async fn run(args: PinyinArgs) -> Result<(), GlossError> {
    let source = std::fs::read_to_string(&args.words)?;
    let selected: Vec<String> = source
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_owned)
        .collect();
    if selected.is_empty() {
        return Err(GlossError::NoWords(args.words.clone()));
    }
    let store = Arc::new(Store::<PinyinEntry>::open(&args.out)?);
    let mut pending: Vec<String> = selected
        .iter()
        .filter(|w| !store.contains(w))
        .cloned()
        .collect();
    if let Some(limit) = args.limit {
        pending.truncate(limit);
    }
    tracing::info!(
        selected = selected.len(),
        already_done = store.len(),
        pending = pending.len(),
        model = %args.model,
        "开始标注拼音"
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
        Arc::new(PinyinTask),
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
    fn cleans_tones_and_rejects_wrong_syllable_counts() {
        assert_eq!(clean_pinyin("chong2 qing4", 2).unwrap(), ["chong", "qing"]);
        assert_eq!(clean_pinyin("nv3'ren2", 2).unwrap(), ["nv", "ren"]);
        assert_eq!(clean_pinyin("LÜ4", 1).unwrap(), ["lv"]);
        assert!(clean_pinyin("chong2", 2).is_none());
        assert!(clean_pinyin("chong2 qing4 shi4", 2).is_none());
        assert!(clean_pinyin("chong-2 qing4", 2).is_none());
    }

    #[test]
    fn parses_only_requested_words() {
        let words = vec!["重庆".to_owned(), "长沙".to_owned()];
        let reply = r#"{"items":[{"w":"重庆","py":"chong2 qing4"},{"w":"北京","py":"bei3 jing1"},{"w":"长沙","py":"chang2"}]}"#;
        let entries = PinyinTask.parse_reply(reply, &words).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].word, "重庆");
    }
}
