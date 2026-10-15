//! 越南语输入词表：从 `glossary-vi.tsv` 的越南语释义抽取词/短语，并反推 Telex 编码。

use std::collections::BTreeMap;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::error::ConvertError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tone {
    None,
    Acute,
    Grave,
    Hook,
    Tilde,
    Dot,
}

impl Tone {
    const fn key(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Acute => "s",
            Self::Grave => "f",
            Self::Hook => "r",
            Self::Tilde => "x",
            Self::Dot => "j",
        }
    }
}

#[derive(Debug, Clone)]
struct Row {
    text: String,
    telex: String,
    frequency: u32,
}

pub fn convert(input: &Path, out_dir: &Path) -> Result<(), ConvertError> {
    std::fs::create_dir_all(out_dir)?;
    let source = std::fs::read_to_string(input)?;
    let mut words: BTreeMap<String, Row> = BTreeMap::new();
    let mut phrases: BTreeMap<String, Row> = BTreeMap::new();
    for (line_no, raw) in source.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let base_frequency = 100_000u32.saturating_sub((line_no as u32).min(90_000));
        for field in line.split('\t').skip(1) {
            let Some(text) = clean_sense(field) else {
                continue;
            };
            let Some(telex) = telex_phrase(&text) else {
                continue;
            };
            if text.contains(' ') {
                insert_row(&mut phrases, text.clone(), telex, base_frequency);
                for word in text.split_whitespace() {
                    if let Some(telex) = telex_word(word) {
                        insert_row(&mut words, word.to_owned(), telex, base_frequency);
                    }
                    let lower = word.to_lowercase();
                    if lower != word
                        && let Some(telex) = telex_word(&lower)
                    {
                        insert_row(&mut words, lower, telex, base_frequency);
                    }
                }
            } else {
                insert_row(&mut words, text, telex, base_frequency);
            }
        }
    }
    write_rows(&out_dir.join("words.tsv"), "词", words.into_values())?;
    write_rows(&out_dir.join("phrases.tsv"), "短语", phrases.into_values())?;
    Ok(())
}

fn insert_row(rows: &mut BTreeMap<String, Row>, text: String, telex: String, frequency: u32) {
    rows.entry(text.clone())
        .and_modify(|row| row.frequency = row.frequency.saturating_add(1))
        .or_insert(Row {
            text,
            telex,
            frequency,
        });
}

fn write_rows(
    path: &Path,
    label: &str,
    rows: impl Iterator<Item = Row>,
) -> Result<(), ConvertError> {
    let mut rows: Vec<Row> = rows.collect();
    rows.sort_by(|a, b| {
        b.frequency
            .cmp(&a.frequency)
            .then_with(|| a.telex.cmp(&b.telex))
            .then_with(|| a.text.cmp(&b.text))
    });
    let mut file = BufWriter::new(std::fs::File::create(path)?);
    writeln!(
        file,
        "# 由 qingjian-dict-convert vietnamese 从 glossary-vi.tsv 生成。{label}\ttelex\t词频"
    )?;
    for row in &rows {
        writeln!(file, "{}\t{}\t{}", row.text, row.telex, row.frequency)?;
    }
    file.flush()?;
    tracing::info!(path = %path.display(), entries = rows.len(), "越南语{label}表已写出");
    Ok(())
}

fn clean_sense(text: &str) -> Option<String> {
    let mut text = text.trim();
    let lower = text.to_ascii_lowercase();
    for prefix in [
        "adj.", "adv.", "art.", "aux.", "clf.", "conj.", "int.", "n.", "num.", "part.", "prep.",
        "pron.", "v.",
    ] {
        if lower.starts_with(prefix) {
            text = text[prefix.len()..].trim();
            break;
        }
    }
    let text = text
        .trim_matches(|c: char| c.is_ascii_punctuation() && c != '-' && c != '\'')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if text.is_empty()
        || text.chars().any(is_rejected_char)
        || is_ascii_non_vietnamese_word(&text)
        || !text.chars().any(|c| c.is_alphabetic())
    {
        return None;
    }
    Some(text)
}

fn is_ascii_non_vietnamese_word(text: &str) -> bool {
    if text.contains(' ')
        || !text
            .bytes()
            .all(|b| b.is_ascii_alphabetic() || b == b'-' || b == b'\'')
    {
        return false;
    }
    !matches!(
        text.to_ascii_lowercase().as_str(),
        "anh"
            | "em"
            | "la"
            | "nam"
            | "ai"
            | "an"
            | "ba"
            | "bo"
            | "co"
            | "cho"
            | "con"
            | "di"
            | "do"
            | "ga"
            | "me"
            | "no"
            | "to"
            | "va"
            | "ve"
            | "vi"
    )
}

fn is_rejected_char(c: char) -> bool {
    c.is_ascii_digit()
        || matches!(
            c,
            '/' | '\\' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';' | ':' | '，'
        )
        || ('\u{4e00}'..='\u{9fff}').contains(&c)
}

fn telex_phrase(text: &str) -> Option<String> {
    text.split_whitespace()
        .map(telex_word)
        .collect::<Option<Vec<_>>>()
        .map(|parts| parts.join(" "))
}

fn telex_word(text: &str) -> Option<String> {
    let mut out = String::new();
    let mut tone = Tone::None;
    for ch in text.chars() {
        match ch {
            '-' | '\'' => out.push(ch),
            c if c.is_ascii_alphabetic() => out.push(c.to_ascii_lowercase()),
            c => {
                let (keys, next_tone) = char_telex(c)?;
                out.push_str(keys);
                if next_tone != Tone::None {
                    if tone != Tone::None && tone != next_tone {
                        return None;
                    }
                    tone = next_tone;
                }
            }
        }
    }
    out.push_str(tone.key());
    Some(out)
}

fn char_telex(c: char) -> Option<(&'static str, Tone)> {
    Some(match c {
        'á' | 'Á' => ("a", Tone::Acute),
        'à' | 'À' => ("a", Tone::Grave),
        'ả' | 'Ả' => ("a", Tone::Hook),
        'ã' | 'Ã' => ("a", Tone::Tilde),
        'ạ' | 'Ạ' => ("a", Tone::Dot),
        'ă' | 'Ă' => ("aw", Tone::None),
        'ắ' | 'Ắ' => ("aw", Tone::Acute),
        'ằ' | 'Ằ' => ("aw", Tone::Grave),
        'ẳ' | 'Ẳ' => ("aw", Tone::Hook),
        'ẵ' | 'Ẵ' => ("aw", Tone::Tilde),
        'ặ' | 'Ặ' => ("aw", Tone::Dot),
        'â' | 'Â' => ("aa", Tone::None),
        'ấ' | 'Ấ' => ("aa", Tone::Acute),
        'ầ' | 'Ầ' => ("aa", Tone::Grave),
        'ẩ' | 'Ẩ' => ("aa", Tone::Hook),
        'ẫ' | 'Ẫ' => ("aa", Tone::Tilde),
        'ậ' | 'Ậ' => ("aa", Tone::Dot),
        'é' | 'É' => ("e", Tone::Acute),
        'è' | 'È' => ("e", Tone::Grave),
        'ẻ' | 'Ẻ' => ("e", Tone::Hook),
        'ẽ' | 'Ẽ' => ("e", Tone::Tilde),
        'ẹ' | 'Ẹ' => ("e", Tone::Dot),
        'ê' | 'Ê' => ("ee", Tone::None),
        'ế' | 'Ế' => ("ee", Tone::Acute),
        'ề' | 'Ề' => ("ee", Tone::Grave),
        'ể' | 'Ể' => ("ee", Tone::Hook),
        'ễ' | 'Ễ' => ("ee", Tone::Tilde),
        'ệ' | 'Ệ' => ("ee", Tone::Dot),
        'í' | 'Í' => ("i", Tone::Acute),
        'ì' | 'Ì' => ("i", Tone::Grave),
        'ỉ' | 'Ỉ' => ("i", Tone::Hook),
        'ĩ' | 'Ĩ' => ("i", Tone::Tilde),
        'ị' | 'Ị' => ("i", Tone::Dot),
        'ó' | 'Ó' => ("o", Tone::Acute),
        'ò' | 'Ò' => ("o", Tone::Grave),
        'ỏ' | 'Ỏ' => ("o", Tone::Hook),
        'õ' | 'Õ' => ("o", Tone::Tilde),
        'ọ' | 'Ọ' => ("o", Tone::Dot),
        'ô' | 'Ô' => ("oo", Tone::None),
        'ố' | 'Ố' => ("oo", Tone::Acute),
        'ồ' | 'Ồ' => ("oo", Tone::Grave),
        'ổ' | 'Ổ' => ("oo", Tone::Hook),
        'ỗ' | 'Ỗ' => ("oo", Tone::Tilde),
        'ộ' | 'Ộ' => ("oo", Tone::Dot),
        'ơ' | 'Ơ' => ("ow", Tone::None),
        'ớ' | 'Ớ' => ("ow", Tone::Acute),
        'ờ' | 'Ờ' => ("ow", Tone::Grave),
        'ở' | 'Ở' => ("ow", Tone::Hook),
        'ỡ' | 'Ỡ' => ("ow", Tone::Tilde),
        'ợ' | 'Ợ' => ("ow", Tone::Dot),
        'ú' | 'Ú' => ("u", Tone::Acute),
        'ù' | 'Ù' => ("u", Tone::Grave),
        'ủ' | 'Ủ' => ("u", Tone::Hook),
        'ũ' | 'Ũ' => ("u", Tone::Tilde),
        'ụ' | 'Ụ' => ("u", Tone::Dot),
        'ư' | 'Ư' => ("uw", Tone::None),
        'ứ' | 'Ứ' => ("uw", Tone::Acute),
        'ừ' | 'Ừ' => ("uw", Tone::Grave),
        'ử' | 'Ử' => ("uw", Tone::Hook),
        'ữ' | 'Ữ' => ("uw", Tone::Tilde),
        'ự' | 'Ự' => ("uw", Tone::Dot),
        'ý' | 'Ý' => ("y", Tone::Acute),
        'ỳ' | 'Ỳ' => ("y", Tone::Grave),
        'ỷ' | 'Ỷ' => ("y", Tone::Hook),
        'ỹ' | 'Ỹ' => ("y", Tone::Tilde),
        'ỵ' | 'Ỵ' => ("y", Tone::Dot),
        'đ' | 'Đ' => ("dd", Tone::None),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_vietnamese_to_telex() {
        assert_eq!(telex_word("vẫy"), Some("vaayx".to_owned()));
        assert_eq!(telex_word("tiếng"), Some("tieengs".to_owned()));
        assert_eq!(
            telex_phrase("tiếng Việt"),
            Some("tieengs vieetj".to_owned())
        );
    }

    #[test]
    fn cleans_part_of_speech_prefixes() {
        assert_eq!(clean_sense("v. vẫy"), Some("vẫy".to_owned()));
        assert_eq!(clean_sense("adj. sạch sẽ"), Some("sạch sẽ".to_owned()));
    }

    #[test]
    fn filters_plain_english_words_but_keeps_common_vietnamese_ascii_words() {
        assert_eq!(clean_sense("n. anime"), None);
        assert_eq!(clean_sense("n. Anthony"), None);
        assert_eq!(clean_sense("pron. anh"), Some("anh".to_owned()));
    }
}
