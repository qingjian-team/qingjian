//! 提示词与回复解析。

use qingjian_core::PartOfSpeech;
use serde::Deserialize;

use crate::entry::{GlossEntry, JapaneseSense};

/// 每种语言最多留几个译词。
const MAX_SENSES: usize = 2;

/// 单个英文译词最长几个字节：再长就是解释不是译词。
const MAX_ENGLISH_BYTES: usize = 40;

/// 单个日文译词最多几个字符。
const MAX_JAPANESE_CHARS: usize = 16;

/// 单个越南文译词最多几个字符。
const MAX_VIETNAMESE_CHARS: usize = 36;

pub const SYSTEM_PROMPT: &str = "你是多语词典编纂者。给每个中文词写最简短的英文、日文和越南文对应词，供拼音输入法在候选词旁边一行显示，所以只要词、不要解释。\n\
规则：\n\
- pos：这个中文词最主要的词性，只能是 n. v. adj. adv. pron. prep. conj. num. m. part. int. phr. 之一（m. 量词，part. 助词，phr. 短语或成语）。\n\
- en：1 到 2 个最贴切的英文对应词，按常用度排；每个不超过 3 个英文单词；动词用原形，名词用单数；不要括号、不要解释、不要例句。\n\
- ja：1 到 2 个最贴切的日文对应词，按常用度排；每个给 t（通常写法，汉字假名混写）和 r（t 的完整读音，只用平假名，外来语用片假名）；サ変动词写成「〜する」；不要解释。\n\
- vi：1 到 2 个最贴切的越南文对应词，按常用度排；每个不超过 4 个越南文单词；动词用原形，名词用单数或最常用形式；使用越南语变音符；不要括号、不要解释、不要例句。\n\
- 人名地名等专名照译（日文用惯用写法）；多义词只取最常用的义项；网络用语、方言也要给最接近的说法；没有把握也要给最可能的答案，不要留空。\n\
输出严格的 JSON：{\"items\":[{\"w\":\"开发\",\"pos\":\"v.\",\"en\":[\"develop\",\"exploit\"],\"ja\":[{\"t\":\"開発する\",\"r\":\"かいはつする\"}],\"vi\":[\"phát triển\",\"khai thác\"]}]}。\n\
items 与输入的词一一对应、顺序一致、每个词恰好一项，w 必须原样照抄输入的词。";

pub const VIETNAMESE_ONLY_SYSTEM_PROMPT: &str = "你是汉越词典编纂者。给每个中文词写最简短的越南文对应词，供拼音输入法在候选词旁边一行显示，所以只要词、不要解释。\n\
规则：\n\
- pos：这个中文词最主要的词性，只能是 n. v. adj. adv. pron. prep. conj. num. m. part. int. phr. 之一（m. 量词，part. 助词，phr. 短语或成语）。\n\
- vi：1 到 2 个最贴切的越南文对应词，按常用度排；每个不超过 4 个越南文单词；动词用原形，名词用单数或最常用形式；使用越南语变音符；不要括号、不要解释、不要例句。\n\
- 人名地名等专名照译或用越南语常用写法；多义词只取最常用的义项；网络用语、方言也要给最接近的说法；没有把握也要给最可能的答案，不要留空。\n\
输出严格的 JSON：{\"items\":[{\"w\":\"开发\",\"pos\":\"v.\",\"vi\":[\"phát triển\",\"khai thác\"]}]}。\n\
items 与输入的词一一对应、顺序一致、每个词恰好一项，w 必须原样照抄输入的词。";

/// 本次生成的学习语言集合。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlossLanguages {
    en: bool,
    ja: bool,
    vi: bool,
}

impl GlossLanguages {
    pub fn parse(raw: &[String]) -> Self {
        let mut languages = Self {
            en: false,
            ja: false,
            vi: false,
        };
        for language in raw {
            match language.trim().to_ascii_lowercase().as_str() {
                "en" | "english" => languages.en = true,
                "ja" | "jp" | "japanese" => languages.ja = true,
                "vi" | "vi-vn" | "vietnamese" => languages.vi = true,
                other => tracing::warn!(language = other, "不认识的生成语言，跳过"),
            }
        }
        if !languages.en && !languages.ja && !languages.vi {
            tracing::warn!("没有可生成的语言，按 en,ja,vi");
            return Self::all();
        }
        languages
    }

    pub fn all() -> Self {
        Self {
            en: true,
            ja: true,
            vi: true,
        }
    }

    pub fn label(self) -> &'static str {
        match (self.en, self.ja, self.vi) {
            (true, true, true) => "en,ja,vi",
            (true, true, false) => "en,ja",
            (true, false, true) => "en,vi",
            (false, true, true) => "ja,vi",
            (true, false, false) => "en",
            (false, true, false) => "ja",
            (false, false, true) => "vi",
            (false, false, false) => "",
        }
    }
}

pub fn system_prompt(languages: GlossLanguages) -> &'static str {
    if languages == GlossLanguages::all() {
        SYSTEM_PROMPT
    } else if languages
        == (GlossLanguages {
            en: false,
            ja: false,
            vi: true,
        })
    {
        VIETNAMESE_ONLY_SYSTEM_PROMPT
    } else {
        SYSTEM_PROMPT
    }
}

/// 用户消息：一行一个词。
pub fn user_prompt(words: &[String]) -> String {
    let mut text = String::from("词：\n");
    for word in words {
        text.push_str(word);
        text.push('\n');
    }
    text
}

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
    en: Vec<String>,

    #[serde(default)]
    ja: Vec<RawJapanese>,

    #[serde(default)]
    vi: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawJapanese {
    t: String,

    #[serde(default)]
    r: Option<String>,
}

/// 解析回复：只收请求过的词，每个词一条（重复取第一条），译词逐个校验、清洗；两种语言都空的丢掉。
pub fn parse_reply_for_languages(
    content: &str,
    words: &[String],
    languages: GlossLanguages,
) -> Result<Vec<GlossEntry>, serde_json::Error> {
    let reply: RawReply = serde_json::from_str(content.trim())?;
    let mut entries: Vec<GlossEntry> = Vec::with_capacity(words.len());
    for item in reply.items {
        let word = item.w.trim();
        if !words.iter().any(|w| w == word) || entries.iter().any(|e| e.word == word) {
            continue;
        }
        let entry = GlossEntry {
            word: word.to_owned(),
            pos: item.pos.as_deref().and_then(normalize_pos),
            en: if languages.en {
                clean_english(item.en)
            } else {
                Vec::new()
            },
            ja: if languages.ja {
                clean_japanese(item.ja)
            } else {
                Vec::new()
            },
            vi: if languages.vi {
                clean_latin_language(item.vi, MAX_VIETNAMESE_CHARS, 5)
            } else {
                Vec::new()
            },
        };
        if entry.is_useful() {
            entries.push(entry);
        }
    }
    Ok(entries)
}

fn clean_latin_language(raw: Vec<String>, max_chars: usize, max_words: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for text in raw {
        let text = text
            .trim()
            .trim_end_matches(['.', ';', ',', '。', '；', '，'])
            .trim();
        let ok = !text.is_empty()
            && text.chars().count() <= max_chars
            && text.split_whitespace().count() <= max_words
            && text.chars().all(is_latin_word_char);
        if ok && !out.iter().any(|o| o.eq_ignore_ascii_case(text)) {
            out.push(text.to_owned());
        }
        if out.len() == MAX_SENSES {
            break;
        }
    }
    out
}

fn is_latin_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(c, ' ' | '-' | '\'' | '.' | '/')
        || (matches!(c as u32, 0x00C0..=0x024F | 0x1E00..=0x1EFF) && c.is_alphabetic())
}

/// 词性统一成 Core 认得的缩写。
pub fn normalize_pos(raw: &str) -> Option<String> {
    raw.parse::<PartOfSpeech>()
        .ok()
        .map(|pos| pos.abbreviation().to_owned())
}

fn clean_english(raw: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for text in raw {
        let text = text.trim().trim_end_matches(['.', ';', ',']).trim();
        let ok = !text.is_empty()
            && text.len() <= MAX_ENGLISH_BYTES
            && text.split_whitespace().count() <= 4
            && text
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '\'' | '.' | '/'));
        if ok && !out.iter().any(|o| o.eq_ignore_ascii_case(text)) {
            out.push(text.to_owned());
        }
        if out.len() == MAX_SENSES {
            break;
        }
    }
    out
}

fn clean_japanese(raw: Vec<RawJapanese>) -> Vec<JapaneseSense> {
    let mut out: Vec<JapaneseSense> = Vec::new();
    for item in raw {
        let text = item.t.trim();
        if text.is_empty()
            || text.chars().count() > MAX_JAPANESE_CHARS
            || text.contains(['(', '（', '、', '，', ','])
            || out.iter().any(|o| o.text == text)
        {
            continue;
        }
        let reading = item
            .r
            .as_deref()
            .map(str::trim)
            .filter(|r| !r.is_empty() && r.chars().all(is_kana));
        out.push(JapaneseSense {
            text: text.to_owned(),
            reading: reading.map(str::to_owned),
        });
        if out.len() == MAX_SENSES {
            break;
        }
    }
    out
}

/// 平假名、片假名（含长音、中点）与表示接续的波浪线。
pub fn is_kana(c: char) -> bool {
    matches!(c, '\u{3041}'..='\u{309f}' | '\u{30a0}'..='\u{30ff}' | '〜' | '～' | ' ')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_cleans_a_reply() {
        let words = vec!["开发".to_owned(), "你好".to_owned(), "的".to_owned()];
        let content = r#"{"items":[
            {"w":"开发","pos":"verb","en":["develop","exploit (a resource)","to develop something big here"],"ja":[{"t":"開発する","r":"かいはつする"},{"t":"開発","r":"開発"}],"vi":["phát triển","开发"]},
            {"w":"你好","pos":"int.","en":["hello"],"ja":[{"t":"こんにちは","r":"こんにちは"}],"vi":["xin chào"]},
            {"w":"你好","pos":"int.","en":["hi"],"ja":[],"vi":["chào"]},
            {"w":"没请求的","pos":"n.","en":["nope"],"ja":[],"vi":["không"]},
            {"w":"的","pos":"part.","en":[],"ja":[],"vi":[]}
        ]}"#;
        let entries = parse_reply_for_languages(content, &words, GlossLanguages::all()).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].pos.as_deref(), Some("v."));
        assert_eq!(entries[0].en, ["develop"]);
        assert_eq!(entries[0].ja.len(), 2);
        assert_eq!(entries[0].ja[0].reading.as_deref(), Some("かいはつする"));
        assert_eq!(entries[0].ja[1].reading, None);
        assert_eq!(entries[0].vi, ["phát triển"]);
        assert_eq!(entries[1].word, "你好");
        assert_eq!(entries[1].en, ["hello"]);
        assert_eq!(entries[1].vi, ["xin chào"]);
        assert!(user_prompt(&words).ends_with("的\n"));
    }

    #[test]
    fn parses_vietnamese_only_reply() {
        let words = vec!["开发".to_owned(), "你好".to_owned()];
        let content = r#"{"items":[
            {"w":"开发","pos":"verb","vi":["phát triển","开发"]},
            {"w":"你好","pos":"int.","vi":["xin chào"]}
        ]}"#;
        let entries = parse_reply_for_languages(
            content,
            &words,
            GlossLanguages {
                en: false,
                ja: false,
                vi: true,
            },
        )
        .unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries[0].en.is_empty());
        assert!(entries[0].ja.is_empty());
        assert_eq!(entries[0].pos.as_deref(), Some("v."));
        assert_eq!(entries[0].vi, ["phát triển"]);
        assert_eq!(entries[1].vi, ["xin chào"]);
        assert!(VIETNAMESE_ONLY_SYSTEM_PROMPT.contains("越南文"));
    }
}
