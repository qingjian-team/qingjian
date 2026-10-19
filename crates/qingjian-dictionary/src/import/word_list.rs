//! 无读音词表的导入：每行一个词（可带 `词<Tab>权重`），读音从参考词库逐字派生。
//!
//! 面向「只有词、没有拼音」的一类词库（批量导入同学名字 / 路名 / 楼宇名这种）：参考词库
//! （随包基础词库）里每个单字的读音就是派生来源。一个词每个字取一条读音凑成一个组合，
//! **多音字把每种组合各写一条**（`美露莘` 会同时得到 `mei lu shen` 与 `mei lu xin`），
//! 哪个是用户想要的交给排序，导入器不做取舍。
//!
//! 单字读音数与整词组合数各有上限：超限的词整条跳过并计入报告，不静默丢词；字不在参考词库里的
//! 词同样跳过并计入报告（一个字的读音都推不出来，写进去也查不到）。

use std::collections::{HashMap, HashSet};

use crate::dictionary::Dictionary;

/// 一个单字最多留几条读音。
const MAX_READINGS_PER_CHAR: usize = 4;

/// 一个词最多留几个读音组合，超过就整条跳过。
const MAX_VARIANTS: usize = 16;

/// 解析无读音词表：每行 `词` 或 `词<Tab>权重`，`#` 注释与空行忽略，Rime 的 `---` … `...` 头一并忽略。
///
/// 第二列不是整数的行跳过——那是带拼音的词库，该走 [`super::import`]。同一个词只留第一次出现的权重。
pub fn parse_words(text: &str) -> Vec<(String, u32)> {
    let mut words = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut in_header = false;
    let mut started = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // `---` … `...` 之间的 YAML 头整段跳过
        if line == "---" && !started {
            in_header = true;
            continue;
        }
        if line == "..." && in_header {
            in_header = false;
            continue;
        }
        if in_header {
            continue;
        }
        // 第一个词之前，`name:` / `sort:` 这类 YAML 键跳过（没有 `---` 的头）
        if !started && line.contains(':') && !line.contains('\t') {
            continue;
        }
        started = true;
        let mut fields = line.split('\t');
        let Some(word) = fields.next().filter(|word| !word.is_empty()) else {
            continue;
        };
        let weight = match fields.next().map(str::trim) {
            None | Some("") => 1,
            Some(value) => match value.parse::<u32>() {
                Ok(weight) => weight,
                Err(_) => continue,
            },
        };
        if seen.insert(word.to_owned()) {
            words.push((word.to_owned(), weight));
        }
    }
    words
}

/// 派生结果。
pub struct Derived {
    /// 青简 TSV（`词\t拼音\t词频`，一个读音组合一行）。
    pub tsv: String,

    /// 派生统计。
    pub report: Report,
}

/// 派生统计。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// 派生出读音的词数。
    pub words: usize,

    /// 写出的词条数（多音字一个词好几条）。
    pub entries: usize,

    /// 跳过的词：有字不在参考词库里，推不出读音。
    pub unknown: Vec<String>,

    /// 跳过的词：读音组合数超过上限。
    pub overflowed: Vec<String>,
}

/// 参考词库里的单字读音：字 → 读音（按词频降序、去重、封顶）。
struct Readings {
    chars: HashMap<char, Vec<String>>,
}

impl Readings {
    /// 只取单字条目；一个字多条读音时留词频高的前 [`MAX_READINGS_PER_CHAR`] 条。
    fn build(reference: &Dictionary) -> Self {
        let mut collected: HashMap<char, Vec<(u32, String)>> = HashMap::new();
        for entry in reference.entries() {
            let mut chars = entry.text.chars();
            let (Some(ch), None) = (chars.next(), chars.next()) else {
                continue;
            };
            if entry.pinyin.contains(' ') {
                continue;
            }
            collected
                .entry(ch)
                .or_default()
                .push((entry.frequency, entry.pinyin.to_owned()));
        }
        let chars = collected
            .into_iter()
            .map(|(ch, mut readings)| {
                readings.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
                let mut seen = HashSet::new();
                let mut kept = Vec::new();
                for (_, reading) in readings {
                    if seen.insert(reading.clone()) && kept.len() < MAX_READINGS_PER_CHAR {
                        kept.push(reading);
                    }
                }
                (ch, kept)
            })
            .collect();
        Self { chars }
    }

    fn of(&self, ch: char) -> &[String] {
        self.chars.get(&ch).map_or(&[], Vec::as_slice)
    }
}

/// 按参考词库给 `words` 逐字派生读音，多音字把每种组合各写一条。
pub fn derive(words: &[(String, u32)], reference: &Dictionary) -> Derived {
    let readings = Readings::build(reference);
    let mut tsv = String::new();
    let mut report = Report {
        words: 0,
        entries: 0,
        unknown: Vec::new(),
        overflowed: Vec::new(),
    };
    for (word, weight) in words {
        let mut combos: Vec<String> = vec![String::new()];
        let mut unreadable = false;
        let mut overflow = false;
        for ch in word.chars() {
            let choices = readings.of(ch);
            if choices.is_empty() {
                unreadable = true;
                break;
            }
            if combos.len() * choices.len() > MAX_VARIANTS {
                overflow = true;
                break;
            }
            let mut next = Vec::with_capacity(combos.len() * choices.len());
            for prefix in &combos {
                for syllable in choices {
                    next.push(if prefix.is_empty() {
                        syllable.clone()
                    } else {
                        format!("{prefix} {syllable}")
                    });
                }
            }
            combos = next;
        }
        if unreadable {
            report.unknown.push(word.clone());
            continue;
        }
        if overflow {
            report.overflowed.push(word.clone());
            continue;
        }
        for pinyin in &combos {
            tsv.push_str(word);
            tsv.push('\t');
            tsv.push_str(pinyin);
            tsv.push('\t');
            tsv.push_str(&weight.to_string());
            tsv.push('\n');
            report.entries += 1;
        }
        report.words += 1;
    }
    Derived { tsv, report }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(entries: &[(&str, &str, u32)]) -> Dictionary {
        let source: String = entries
            .iter()
            .map(|(text, pinyin, frequency)| format!("{text}\t{pinyin}\t{frequency}\n"))
            .collect();
        Dictionary::parse(&source).unwrap()
    }

    #[test]
    fn reads_plain_word_lists_and_weights() {
        let words = parse_words("# 名字\n张三\n李四\t500\n\n王五\t\n");
        assert_eq!(
            words,
            [
                ("张三".to_owned(), 1),
                ("李四".to_owned(), 500),
                ("王五".to_owned(), 1)
            ]
        );
        // 同一个词只留第一次
        assert_eq!(parse_words("张三\n张三\t500\n").len(), 1);
    }

    #[test]
    fn skips_header_of_rime_word_lists() {
        let words = parse_words(
            "# Rime dictionary\n---\nname: names\nversion: \"1\"\nsort: by_weight\n...\n张三\n李四\t7\n",
        );
        assert_eq!(words, [("张三".to_owned(), 1), ("李四".to_owned(), 7)]);
    }

    #[test]
    fn skips_lines_whose_second_column_is_a_reading() {
        // 第二列是拼音 → 那是带读音的词库，本解析器不认，跳过；纯词表里它才不会误判
        assert_eq!(parse_words("张三\tzhang san\t100\n李四\n").len(), 1);
    }

    #[test]
    fn derives_reading_for_each_char() {
        let dictionary = reference(&[("张", "zhang", 900), ("三", "san", 900)]);
        let derived = derive(&[("张三".to_owned(), 3)], &dictionary);
        assert_eq!(derived.tsv, "张三\tzhang san\t3\n");
        assert_eq!(derived.report.words, 1);
        assert_eq!(derived.report.entries, 1);
    }

    #[test]
    fn keeps_every_reading_combination_of_a_polyphonic_char() {
        // 莘 有 shen / xin 两条读音 → 两种组合都写进去，用户打哪个都出得来
        let dictionary = reference(&[
            ("美", "mei", 900),
            ("露", "lu", 900),
            ("莘", "shen", 900),
            ("莘", "xin", 10),
        ]);
        let derived = derive(&[("美露莘".to_owned(), 1)], &dictionary);
        assert_eq!(
            derived.tsv,
            "美露莘\tmei lu shen\t1\n美露莘\tmei lu xin\t1\n"
        );
        assert_eq!(derived.report.entries, 2);
    }

    #[test]
    fn puts_the_more_frequent_reading_first() {
        let dictionary = reference(&[("莘", "shen", 900), ("莘", "xin", 10)]);
        let derived = derive(&[("莘".to_owned(), 1)], &dictionary);
        assert!(derived.tsv.starts_with("莘\tshen"));
    }

    #[test]
    fn skips_words_whose_char_is_not_in_the_reference() {
        let dictionary = reference(&[("三", "san", 900)]);
        let derived = derive(&[("囧三".to_owned(), 1)], &dictionary);
        assert!(derived.tsv.is_empty());
        assert_eq!(derived.report.unknown, ["囧三".to_owned()]);
        assert_eq!(derived.report.entries, 0);
    }

    #[test]
    fn skips_words_with_too_many_reading_combinations() {
        // 五字词、每字两条读音 → 32 种组合，超过 MAX_VARIANTS，整条跳进报告
        let dictionary = reference(&[
            ("甲", "jia", 900),
            ("甲", "xia", 10),
            ("乙", "yi", 900),
            ("乙", "er", 10),
        ]);
        let derived = derive(&[("甲乙甲乙甲".to_owned(), 1)], &dictionary);
        assert!(derived.tsv.is_empty());
        assert_eq!(derived.report.overflowed, ["甲乙甲乙甲".to_owned()]);
    }

    #[test]
    fn caps_readings_per_char_by_frequency() {
        let dictionary = reference(&[
            ("和", "he", 900),
            ("和", "huo", 800),
            ("和", "hu", 700),
            ("和", "huan", 600),
            ("和", "han", 500),
        ]);
        let derived = derive(&[("和".to_owned(), 1)], &dictionary);
        assert_eq!(derived.report.entries, MAX_READINGS_PER_CHAR);
        assert!(derived.tsv.starts_with("和\the\t"));
    }
}
