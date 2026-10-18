//! `--mine-log`：从输入日志挖词库缺词。
//!
//! 每条上屏记录带着「敲的键、切分、上屏文字」三元组，把上屏文字按语言模型切词，
//! 落成连续单字的段（切不动多字词的地方）就是要找的候选：出现够多次的列成
//! `词\t拼音\t次数` 清单，人工过目后并进词库或用户词。词与读音都在日志里现成，
//! 不需要再标注。纯离线，只读日志。
//!
//! 加 `--mine-apply` 时不再只列清单：过阈值的**全拼**行（读音每段都是完整音节）直接
//! `Learner::learn_word` 进用户词，**简拼**行（读音是敲的声母字母）单独列出等人工补全拼。
//! 是否落盘沿用 `--user-dict` 语义：给了写回同目录 `user-words.tsv`，不给就只打印预演。

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::io::BufRead;
use std::path::{Path, PathBuf};

use qingjian_core::parser;
use qingjian_core::sentence::{self, LanguageModel, NoLanguageModel};
use qingjian_core::{InputLogEntry, InputSource, Learner};
use qingjian_dictionary::Dictionary;
use qingjian_learning::FrequencyLearner;
use qingjian_lm::BigramModel;

use crate::args::Args;
use crate::error::CliError;
use crate::replay::line::Line;

/// 词库文本的集合，判断一个词「词库里有没有」。
fn dictionary_texts(dictionary: &Dictionary) -> HashSet<&str> {
    dictionary.entries().map(|entry| entry.text).collect()
}

/// 挖出的一个候选：出现次数，以及日志里的读音切片（一音节一片，`'` 已拆开）。
struct Mined {
    count: u32,

    /// 同一词既来自全拼又来自简拼时留全拼那份（只有全拼能直接进用户词）。
    reading: Vec<String>,
}

/// 挖词主入口：读日志、提取候选，按 `--mine-apply` 决定是列清单还是并进用户词。
pub fn run(args: &Args, path: &Path) -> Result<(), CliError> {
    let dict_path = args.dict.clone().unwrap_or_else(default_dict);
    let dictionary = Dictionary::from_path(dict_path)?;
    let texts = dictionary_texts(&dictionary);
    let model = language_model();
    let mut candidates: HashMap<String, Mined> = HashMap::new();

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

    let mut rows: Vec<(&String, &Mined)> = candidates
        .iter()
        .filter(|(_, mined)| mined.count >= args.mine_min_count)
        .collect();
    rows.sort_by(|a, b| {
        b.1.count
            .cmp(&a.1.count)
            .then_with(|| a.0.len().cmp(&b.0.len()))
            .then_with(|| a.0.cmp(b.0))
    });
    if args.mine_apply {
        apply_rows(args, &rows)
    } else {
        print_list(&rows);
        Ok(())
    }
}

/// 列清单（缺省行为）：`词\t拼音\t次数`，拼音是日志读音拼起来（丢掉音节边界）。
fn print_list(rows: &[(&String, &Mined)]) {
    println!("# 词\t拼音\t次数");
    for (text, mined) in rows {
        println!("{text}\t{}\t{}", mined.reading.join(""), mined.count);
    }
}

/// `--mine-apply`：全拼行并进用户词，简拼行列出来等人工补读音。
///
/// 落盘沿用 `--user-dict` 语义：给了就 `flush` 写回同目录 `user-words.tsv`（保留已有的），
/// 不给只打印预演、不落盘。
fn apply_rows(args: &Args, rows: &[(&String, &Mined)]) -> Result<(), CliError> {
    let mut full: Vec<(&str, Vec<String>)> = Vec::new();
    let mut manual: Vec<(&String, &Mined)> = Vec::new();
    for (text, mined) in rows {
        match full_reading(&mined.reading) {
            Some(syllables) => full.push((text.as_str(), syllables)),
            None => manual.push((text, mined)),
        }
    }

    if let Some(path) = &args.user_dict {
        let mut learner = FrequencyLearner::from_path(path)?;
        for (text, syllables) in &full {
            learner.learn_word(text, syllables);
        }
        learner.flush();
        println!("# 并入用户词 {} 条", full.len());
    } else {
        println!(
            "# 预演：将并入用户词 {} 条（未给 --user-dict，未落盘）",
            full.len()
        );
    }
    if !full.is_empty() {
        println!("词\t音节");
        for (text, syllables) in &full {
            println!("{text}\t{}", syllables.join(" "));
        }
    }
    if !manual.is_empty() {
        println!("# 需人工补读音 {} 条（简拼，读音是敲的字母）", manual.len());
        println!("词\t敲的键\t次数");
        for (text, mined) in &manual {
            println!("{text}\t{}\t{}", mined.reading.join(""), mined.count);
        }
    }
    Ok(())
}

/// 读音是不是「全拼」：每一段都是完整音节（简拼的单字母声母不是）。判定交给 Core 的
/// `parser::segment`，与引擎同一套音节表；用 `'` 连接强制按日志的切分边界走，不会被重新合并。
/// 段数与音节数必须相等（一音节对一字）。全拼行返回音节，简拼行返回 `None`。
fn full_reading(pieces: &[String]) -> Option<Vec<String>> {
    if pieces.is_empty() || pieces.iter().any(|piece| piece.is_empty()) {
        return None;
    }
    let best = parser::segment(&pieces.join("'"))
        .ok()?
        .into_iter()
        .next()?;
    (best.incomplete_count() == 0 && best.syllables.len() == pieces.len()).then(|| {
        best.syllables
            .into_iter()
            .map(|syllable| syllable.text)
            .collect()
    })
}

/// 记一个候选：次数 +1；读音留全拼那份（同一词来自全拼与简拼时）。
fn insert_candidate(out: &mut HashMap<String, Mined>, text: String, reading: &[&str]) {
    let reading: Vec<String> = reading.iter().map(|s| (*s).to_owned()).collect();
    let is_full = full_reading(&reading).is_some();
    match out.entry(text) {
        Entry::Occupied(mut occupied) => {
            let mined = occupied.get_mut();
            mined.count += 1;
            if is_full && full_reading(&mined.reading).is_none() {
                mined.reading = reading;
            }
        }
        Entry::Vacant(vacant) => {
            vacant.insert(Mined { count: 1, reading });
        }
    }
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
    out: &mut HashMap<String, Mined>,
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
        let flush = |unknown: &mut Option<(usize, usize)>, out: &mut HashMap<String, Mined>| {
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
            insert_candidate(
                out,
                candidate,
                &syllables[run_start + start..run_start + start + len],
            );
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
                    insert_candidate(
                        out,
                        word.clone(),
                        &syllables[run_start + offset..run_start + offset + len],
                    );
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
    use clap::Parser;

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
        let got = out.get("花名").unwrap();
        assert_eq!(got.count, 1);
        assert_eq!(got.reading.join(""), "huaming");
        assert!(!out.contains_key("输入"));
        // 连续单字段整段成候选（的 / 法 单看都在词库里，但那是规范字，不是词）
        let mut out = HashMap::new();
        extract("的法", &["de", "fa"], &texts(), &TinyModel, 6, &mut out);
        assert_eq!(out.get("的法").unwrap().reading.join(""), "defa");
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
        assert_eq!(out.get("甲乙丙丁").unwrap().count, 1);
        extract(
            "甲乙丙丁",
            &syllables[..4],
            &texts(),
            &TinyModel,
            4,
            &mut out,
        );
        assert_eq!(out.get("甲乙丙丁").unwrap().count, 2);
    }

    /// 全拼（每段都是完整音节）能直接进用户词；简拼的声母单字母判不出来
    #[test]
    fn distinguishes_full_pinyin_from_abbreviations() {
        let pieces = |parts: &[&str]| parts.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert_eq!(
            full_reading(&pieces(&["kou", "bo"])),
            Some(vec!["kou".to_owned(), "bo".to_owned()])
        );
        // 用户敲法（v 代 ü）也认，交给 Core 的切分归一
        assert_eq!(
            full_reading(&pieces(&["jve"])),
            Some(vec!["jve".to_owned()])
        );
        // 简拼：声母单字母不是完整音节
        assert_eq!(full_reading(&pieces(&["h", "z", "s"])), None);
        assert_eq!(full_reading(&pieces(&["zh", "w"])), None);
        assert_eq!(full_reading(&[]), None);
    }

    /// 同一词既来自全拼又来自简拼时留全拼那份（两种先后顺序都要留全拼）
    #[test]
    fn prefers_full_reading_on_collision() {
        let mut out = HashMap::new();
        insert_candidate(&mut out, "口播".to_owned(), &["k", "b"]);
        insert_candidate(&mut out, "口播".to_owned(), &["kou", "bo"]);
        let got = out.get("口播").unwrap();
        assert_eq!(got.count, 2);
        assert_eq!(got.reading, ["kou".to_owned(), "bo".to_owned()]);

        let mut out = HashMap::new();
        insert_candidate(&mut out, "口播".to_owned(), &["kou", "bo"]);
        insert_candidate(&mut out, "口播".to_owned(), &["k", "b"]);
        assert_eq!(
            out.get("口播").unwrap().reading,
            ["kou".to_owned(), "bo".to_owned()]
        );
    }

    /// `--mine-apply` 落盘：全拼行进 user-words.tsv、简拼行留给人工
    #[test]
    fn apply_writes_full_readings_and_skips_abbreviations() {
        let mut candidates: HashMap<String, Mined> = HashMap::new();
        insert_candidate(&mut candidates, "口播".to_owned(), &["kou", "bo"]);
        insert_candidate(&mut candidates, "傻姑娘".to_owned(), &["s", "g", "n"]);
        let rows: Vec<(&String, &Mined)> = candidates.iter().collect();

        let dir = std::env::temp_dir().join(format!("qingjian-mine-apply-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let user_tsv = dir.join("user.tsv");
        let args = Args::parse_from([
            "qingjian",
            "--mine-log",
            "input-log.jsonl",
            "--mine-apply",
            "--user-dict",
            user_tsv.to_str().unwrap(),
        ]);
        apply_rows(&args, &rows).unwrap();

        let words = std::fs::read_to_string(dir.join("user-words.tsv")).unwrap();
        assert!(words.contains("口播\tkou bo"));
        assert!(!words.contains("傻姑娘"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn han_runs_split_at_non_han_and_keep_offsets() {
        let runs = han_runs("青简hello词库");
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0], (0, "青简".to_owned()));
        assert_eq!(runs[1], (7, "词库".to_owned()));
    }
}
