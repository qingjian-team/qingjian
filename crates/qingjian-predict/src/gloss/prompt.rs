//! 释义兜底的提示词与回复解析。

use qingjian_core::{FilledGloss, Language, PartOfSpeech, Sense, Translation};
use serde::{Deserialize, Serialize};

/// 每个词最多留几条译词。
const MAX_SENSES: usize = Translation::MAX_SENSES;

/// 单条英文译词最长几个字节：再长就是解释不是译词。
const MAX_ENGLISH_BYTES: usize = 40;

/// 单条日文译词最多几个字符。
const MAX_JAPANESE_CHARS: usize = 16;

pub const ENGLISH_SYSTEM_PROMPT: &str = "你是汉英词典编纂者。给每个中文词写最简短的英文对应词，供拼音输入法在候选词旁边一行显示，所以只要词、不要解释。\n\
规则：\n\
- pos：这个中文词最主要的词性，只能是 n. v. adj. adv. pron. prep. conj. num. m. part. int. phr. 之一（m. 量词，part. 助词，phr. 短语或成语）。\n\
- senses：1 到 2 条最贴切的英文对应词，按常用度排；每条不超过 3 个英文单词；动词用原形，名词用单数；不要括号、不要解释、不要例句。\n\
- 人名地名等专名照译；多义词只取最常用的义项；网络用语、方言也要给最接近的说法；没有把握也要给最可能的答案，不要留空。\n\
输出严格的 JSON：{\"items\":[{\"w\":\"开发\",\"pos\":\"v.\",\"senses\":[{\"t\":\"develop\"},{\"t\":\"exploit\"}]}]}。\n\
items 与输入的词一一对应、顺序一致、每个词恰好一项，w 必须原样照抄输入的词。";

pub const JAPANESE_SYSTEM_PROMPT: &str = "你是汉日词典编纂者。给每个中文词写最简短的日文对应词，供拼音输入法在候选词旁边一行显示，所以只要词、不要解释。\n\
规则：\n\
- pos：这个中文词最主要的词性，只能是 n. v. adj. adv. pron. prep. conj. num. m. part. int. phr. 之一（m. 量词，part. 助词，phr. 短语或成语）。\n\
- senses：1 到 2 条最贴切的日文对应词，按常用度排；每条给 t（通常写法，汉字假名混写）和 r（t 的完整读音，只用平假名，外来语用片假名）；サ変动词写成「〜する」；不要解释。\n\
- 人名地名等专名照译（日文用惯用写法）；多义词只取最常用的义项；网络用语、方言也要给最接近的说法；没有把握也要给最可能的答案，不要留空。\n\
输出严格的 JSON：{\"items\":[{\"w\":\"开发\",\"pos\":\"v.\",\"senses\":[{\"t\":\"開発する\",\"r\":\"かいはつする\"}]}]}。\n\
items 与输入的词一一对应、顺序一致、每个词恰好一项，w 必须原样照抄输入的词。";

/// 学习语言对应的系统提示；中文没有（不会请求）。
pub fn system_prompt(language: Language) -> &'static str {
    match language {
        Language::Japanese => JAPANESE_SYSTEM_PROMPT,
        Language::English | Language::Chinese => ENGLISH_SYSTEM_PROMPT,
    }
}

#[derive(Serialize)]
struct UserMessage<'a> {
    words: &'a [String],
}

pub fn user_prompt(words: &[String]) -> String {
    serde_json::to_string(&UserMessage { words }).unwrap_or_default()
}

#[derive(Deserialize)]
struct RawReply {
    #[serde(default)]
    items: Vec<RawItem>,
}

#[derive(Deserialize)]
struct RawItem {
    w: String,

    #[serde(default)]
    pos: Option<String>,

    #[serde(default)]
    senses: Vec<RawSense>,
}

#[derive(Deserialize)]
struct RawSense {
    t: String,

    #[serde(default)]
    r: Option<String>,
}

/// 解析回复：只收请求过的词，每个词一条（重复取第一条），译词逐条清洗，一条都没有的丢掉。解析不了返回空。
pub fn parse_reply(content: &str, language: Language, words: &[String]) -> Vec<FilledGloss> {
    let Ok(reply) = serde_json::from_str::<RawReply>(content.trim()) else {
        return Vec::new();
    };
    let mut out: Vec<FilledGloss> = Vec::with_capacity(words.len());
    for item in reply.items {
        let word = item.w.trim();
        if !words.iter().any(|w| w == word) || out.iter().any(|g| g.word == word) {
            continue;
        }
        let part_of_speech = item
            .pos
            .as_deref()
            .and_then(|pos| pos.trim().parse::<PartOfSpeech>().ok());
        let mut senses: Vec<Sense> = Vec::new();
        for raw in item.senses {
            let Some(text) = clean_text(&raw.t, language) else {
                continue;
            };
            if senses.iter().any(|s| s.text.eq_ignore_ascii_case(&text)) {
                continue;
            }
            let reading = (language == Language::Japanese)
                .then(|| raw.r.as_deref().map(str::trim).filter(|r| is_kana(r)))
                .flatten()
                .map(str::to_owned);
            senses.push(Sense {
                part_of_speech,
                text,
                reading,
                fresh: false,
            });
            if senses.len() == MAX_SENSES {
                break;
            }
        }
        if !senses.is_empty() {
            out.push(FilledGloss {
                word: word.to_owned(),
                translation: Translation::new(language, senses),
            });
        }
    }
    out
}

/// 译词清洗：去首尾标点，太长、含括号的当解释丢掉。
fn clean_text(raw: &str, language: Language) -> Option<String> {
    let text = raw
        .trim()
        .trim_end_matches(['.', ';', ',', '。', '；', '，'])
        .trim();
    if text.is_empty() {
        return None;
    }
    let ok = match language {
        Language::Japanese => {
            text.chars().count() <= MAX_JAPANESE_CHARS
                && !text.contains(['(', '（', '、', '，', ','])
        }
        Language::English | Language::Chinese => {
            text.len() <= MAX_ENGLISH_BYTES
                && text.split_whitespace().count() <= 4
                && text
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '\'' | '.' | '/'))
        }
    };
    ok.then(|| text.to_owned())
}

/// 全是假名（含长音、中点）。
fn is_kana(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| matches!(c as u32, 0x3041..=0x30FF) || c == 'ー' || c == '・')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_english_items_and_drops_explanations() {
        let words = vec!["开放".to_owned(), "测试".to_owned()];
        let content = r#"{"items":[
            {"w":"开放","pos":"adj.","senses":[{"t":"open"},{"t":"open (to the public and everyone)"},{"t":"Open."}]},
            {"w":"没请求","pos":"n.","senses":[{"t":"nope"}]},
            {"w":"测试","pos":"bad","senses":[{"t":""}]}
        ]}"#;
        let filled = parse_reply(content, Language::English, &words);
        assert_eq!(filled.len(), 1);
        assert_eq!(filled[0].word, "开放");
        let senses = filled[0].translation.senses();
        assert_eq!(senses.len(), 1);
        assert_eq!(senses[0].text, "open");
        assert_eq!(senses[0].part_of_speech, Some(PartOfSpeech::Adjective));
        assert!(parse_reply("not json", Language::English, &words).is_empty());
    }

    #[test]
    fn keeps_kana_readings_only_for_japanese() {
        let words = vec!["开发".to_owned()];
        let content = r#"{"items":[{"w":"开发","pos":"v.","senses":[{"t":"開発する","r":"かいはつする"},{"t":"開く","r":"hiraku"}]}]}"#;
        let filled = parse_reply(content, Language::Japanese, &words);
        let senses = filled[0].translation.senses();
        assert_eq!(senses[0].reading.as_deref(), Some("かいはつする"));
        assert_eq!(senses[1].reading, None);
        assert_eq!(filled[0].translation.language, Language::Japanese);
        assert!(user_prompt(&words).contains("开发"));
    }
}
