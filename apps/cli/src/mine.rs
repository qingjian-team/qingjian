//! `--mine-log`：从输入日志挖词库缺词。
//!
//! 每条上屏记录带着「敲的键、切分、上屏文字」三元组，把上屏文字按语言模型切词，
//! 落成连续单字的段（切不动多字词的地方）就是要找的候选：出现够多次的列成
//! `词\t拼音\t次数` 清单，人工过目后并进词库或用户词。词与读音都在日志里现成，
//! 不需要再标注。纯离线，只读日志，不写任何文件。

use std::collections::{HashMap, HashSet};
use std::io::BufRead;
use std::path::{Path, PathBuf};

use qingjian_core::sentence::{self, LanguageModel, NoLanguageModel};
use qingjian_core::{InputLogEntry, InputSource};
use qingjian_dictionary::Dictionary;
use qingjian_lm::BigramModel;

use crate::args::Args;
use crate::error::CliError;
use crate::replay::line::Line;

/// 词库文本的集合，判断一个词「词库里有没有」。
fn dictionary_texts(dictionary: &Dictionary) -> HashSet<&str> {
    dictionary.entries().map(|entry| entry.text).collect()
}

/// 挖词主入口：读日志、提取候选、按次数排序打印。
pub fn run(args: &Args, path: &Path) -> Result<(), CliError> {
    let dict_path = args.dict.clone().unwrap_or_else(default_dict);
    let dictionary = Dictionary::from_path(dict_path)?;
    let texts = dictionary_texts(&dictionary);
    let model = language_model();
    let mut candidates: HashMap<String, (u32, String)> = HashMap::new();

    let file = std::fs::File::open(path)?;
    let mut skipped = 0usize;
    for line in std::io::BufReader::new(file).lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(parsed) = serde_json::from_str::<Line>(line) else {
            skipped += 1;
            continue;
        };
        let InputLogEntry::Commit(commit) = parsed.entry else {
            continue;
        };
        // 只挖词与整句的上屏：云端词读音不可信，原样上屏没有切分
        if !matches!(commit.source, InputSource::Word | InputSource::Sentence) {
            continue;
        }
        let syllables: Vec<&str> = commit.pinyin.split('\'').collect();
        let chars = commit.text.chars().count();
        if syllables.len() != chars || commit.text.is_empty() {
            continue;
        }
        extract(
            &commit.text,
            &syllables,
            &texts,
            &*model,
            args.mine_max_chars,
            &mut candidates,
        );
    }
    if skipped > 0 {
        tracing::warn!(skipped, "日志里有读不了的行，已跳过");
    }

    let mut rows: Vec<(&String, &(u32, String))> = candidates
        .iter()
        .filter(|(_, (count, _))| *count >= args.mine_min_count)
        .collect();
    rows.sort_by(|a, b| {
        b.1.0
            .cmp(&a.1.0)
            .then_with(|| a.0.len().cmp(&b.0.len()))
            .then_with(|| a.0.cmp(b.0))
    });
    println!("# 词\t拼音\t次数");
    for (text, (count, pinyin)) in rows {
        println!("{text}\t{pinyin}\t{count}");
    }
    Ok(())
}

fn default_dict() -> PathBuf {
    crate::args::default_data_file("dict.tsv")
}

/// 与 build_engine 同一套查找：lm.qj 优先，退回两张 TSV；都没有就退化为单字流（挖不出多字词）。
fn language_model() -> Box<dyn LanguageModel> {
    let packed = PathBuf::from("data/generated/lm.qj");
    if packed.is_file()
        && let Ok(model) = BigramModel::from_path(&packed)
    {
        return Box::new(model);
    }
    let unigram = PathBuf::from("data/generated/lm-unigram.tsv");
    let bigram = PathBuf::from("data/generated/lm-bigram.tsv");
    if unigram.is_file()
        && bigram.is_file()
        && let Ok(model) = BigramModel::from_paths(&unigram, &bigram)
    {
        return Box::new(model);
    }
    tracing::warn!("语言模型不可用，挖词退化为单字流，多字词挖不出来");
    Box::new(NoLanguageModel)
}

/// 从一条上屏记录里提取缺词候选。`text` 与 `syllables` 逐字对应（各日志行已校验）。
///
/// 汉字段按语言模型切词，判定看**多字词**：切出来的多字词在词库里就跳过；连续的单字段
/// 是切词失灵的地方，整段并成候选（单字本身都在词库里——规范字——不能按字判）；
/// 模型认识、词库却没有的多字词本身也是候选。长过上限的段不切窗，宁缺勿滥。
/// 候选的读音按字符位置对回拼音切片。
fn extract(
    text: &str,
    syllables: &[&str],
    texts: &HashSet<&str>,
    model: &dyn LanguageModel,
    max_chars: usize,
    out: &mut HashMap<String, (u32, String)>,
) {
    for (run_start, clause) in han_runs(text) {
        // 模型一个多字词都不认识时 segment_text 返回 None——对挖词那正是「整段单字流」，
        // 照单字流处理（整段并成一个候选，长过上限由 flush 滤掉）
        let words = sentence::segment_text(&clause, model)
            .map(|clauses| clauses.into_iter().flatten().collect::<Vec<_>>())
            .unwrap_or_else(|| clause.chars().map(String::from).collect());
        let chars: Vec<char> = clause.chars().collect();
        let mut offset = 0usize;
        let mut unknown: Option<(usize, usize)> = None;
        let flush = |unknown: &mut Option<(usize, usize)>,
                     out: &mut HashMap<String, (u32, String)>| {
            let Some((start, len)) = unknown.take() else {
                return;
            };
            if len < 2 || len > max_chars {
                return;
            }
            let candidate: String = chars[start..start + len].iter().collect();
            // 兜底的单字流里也有词库本来就有的词（模型缺席时整段都是单字）：缺词清单只要词库没有的
            if texts.contains(candidate.as_str()) {
                return;
            }
            let pinyin = syllables[run_start + start..run_start + start + len].join("");
            let entry = out.entry(candidate).or_insert((0, pinyin));
            entry.0 += 1;
        };
        for word in words {
            let len = word.chars().count();
            if len == 1 {
                match unknown {
                    Some((_, ref mut seen)) => *seen += len,
                    None => unknown = Some((offset, len)),
                }
            } else if texts.contains(word.as_str()) {
                flush(&mut unknown, out);
            } else {
                // 模型认识、词库没有的多字词：本身就是一个候选
                flush(&mut unknown, out);
                if len <= max_chars {
                    let pinyin = syllables[run_start + offset..run_start + offset + len].join("");
                    let entry = out.entry(word.clone()).or_insert((0, pinyin));
                    entry.0 += 1;
                }
            }
            offset += len;
        }
        flush(&mut unknown, out);
    }
}

/// 汉字连续段与其在整段文本里的字符起点。口径与 Core `text_segment` 一致：非汉字处断开。
fn han_runs(text: &str) -> Vec<(usize, String)> {
    let mut runs = Vec::new();
    let mut start: Option<usize> = None;
    for (index, c) in text.chars().enumerate() {
        if is_han(c) {
            start.get_or_insert(index);
        } else if let Some(begin) = start.take() {
            runs.push((
                begin,
                text.chars().skip(begin).take(index - begin).collect(),
            ));
        }
    }
    if let Some(begin) = start {
        let total = text.chars().count();
        runs.push((
            begin,
            text.chars().skip(begin).take(total - begin).collect(),
        ));
    }
    runs
}

/// 是不是汉字（与 Core `sentence::text_segment` 同一口径）。
fn is_han(c: char) -> bool {
    matches!(
        c as u32,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x323AF
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 只认识 开发 / 输入 两个多字词的假模型，与 text_segment 的测试同款。
    struct TinyModel;

    impl LanguageModel for TinyModel {
        fn log_prob(&self, _previous: Option<&str>, word: &str) -> Option<f64> {
            match word {
                "开发" => Some(-5.0),
                "输入" => Some(-5.5),
                _ => None,
            }
        }
    }

    fn texts() -> HashSet<&'static str> {
        ["开发", "输入", "青简"].into_iter().collect()
    }

    #[test]
    fn mines_runs_of_single_chars_and_model_known_words() {
        let mut out = HashMap::new();
        // 输入 在词库：跳过；青简 是两个单字连成的段：候选带拼音
        extract(
            "花名输入",
            &["hua", "ming", "shu", "ru"],
            &texts(),
            &TinyModel,
            6,
            &mut out,
        );
        assert_eq!(out.get("花名"), Some(&(1, "huaming".to_owned())));
        assert!(!out.contains_key("输入"));
        // 连续单字段整段成候选（的 / 法 单看都在词库里，但那是规范字，不是词）
        let mut out = HashMap::new();
        extract("的法", &["de", "fa"], &texts(), &TinyModel, 6, &mut out);
        assert_eq!(out.get("的法"), Some(&(1, "defa".to_owned())));
    }

    /// 词库里有的词即使被切成单字流（模型缺席的兜底路径）也不是缺词
    #[test]
    fn skips_dictionary_words_in_the_fallback_run() {
        let mut out = HashMap::new();
        extract("青简", &["qing", "jian"], &texts(), &TinyModel, 6, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn caps_length_and_counts_repeats() {
        let mut out = HashMap::new();
        let syllables = ["a", "b", "c", "d", "e", "f", "g", "h"];
        // 长过上限的段不切窗，整段不要
        extract(
            "甲乙丙丁戊己庚辛",
            &syllables,
            &texts(),
            &TinyModel,
            4,
            &mut out,
        );
        assert!(out.is_empty());
        extract(
            "甲乙丙丁",
            &syllables[..4],
            &texts(),
            &TinyModel,
            4,
            &mut out,
        );
        assert_eq!(out.get("甲乙丙丁").unwrap().0, 1);
        extract(
            "甲乙丙丁",
            &syllables[..4],
            &texts(),
            &TinyModel,
            4,
            &mut out,
        );
        assert_eq!(out.get("甲乙丙丁").unwrap().0, 2);
    }

    #[test]
    fn han_runs_split_at_non_han_and_keep_offsets() {
        let runs = han_runs("青简hello词库");
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0], (0, "青简".to_owned()));
        assert_eq!(runs[1], (7, "词库".to_owned()));
    }
}
