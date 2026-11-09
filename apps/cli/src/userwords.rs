//! `--check-user-words`：按读音核对用户词，清理历史遗留的错读音用户词。
//!
//! 云端词学成用户词时读音以用户敲的拼音为准、再逐字核对词库读音，核不过不学（见 architecture.md
//! 「联想与个人模型的边界」）；但历史上已经学进去的错读音（`我的 wo di` 这类）没有清理入口——
//! 它们按错读音出现在候选里，还会被词图当「同文本不同读音」顶掉正确的整句候选。本模块补这个入口。
//!
//! 主词库读音**无调**，逐字核对判不出「字有多音」（`的` 在 `di` 下本就合法），所以只抓两类可客观判定的：
//! **读音不符**——词在词库里、存的读音不是词库给这个词的读音；**读音可疑**——词库不认识这个词、
//! 但某个字按存的读音在词库里根本不是这个字的读音（等于今天的学习闸门也不会放它进来）。
//! 词库读音与用户词读音都经 `Dictionary` 规范化（音节按 `canonical_syllable` 归一、空格分隔），可直接比。

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;

use crate::args::Args;
use crate::error::CliError;

/// 一行核对结果（`text` / `reading` 自己持有：要在删词前先把手里的借用放掉）。
struct Row {
    text: String,
    /// 用户词里存的读音（空格分音节）。
    reading: String,
    /// 词库侧对照：读音不符给词库给该词的读音，读音可疑给不合的字音。
    detail: String,
}

/// 分档结果。
#[derive(Default)]
struct Report {
    /// 词库有该词，存的读音不是词库给的读音。
    mismatch: Vec<Row>,

    /// 词库不认识该词，且逐字核对不过。
    suspect: Vec<Row>,
}

/// 词库侧的读音参照：整词的读音集合 + 单字的读音集合。
#[derive(Default)]
struct DictReadings {
    /// 词文本 → 词库给它的读音（空格分音节的字符串）。
    words: HashMap<String, HashSet<String>>,

    /// 单字 → 词库给它的音节。
    chars: HashMap<char, HashSet<String>>,
}

/// 核对入口：列出读音对不上的用户词，`--purge-user-words` 时删掉并落盘。
///
/// `Learner::flush` 的契约是「持久化失败只记日志」（输入优先于学习），写没写进磁盘它不报错，
/// 所以删完自己读回 `user-words.tsv` 核实，没删干净就报错、以非零状态退出。
pub fn run(args: &Args, engine: &mut Engine) -> Result<(), CliError> {
    let Some(user) = engine.learner().user_words() else {
        println!("# 没有用户词可核对（未给 --user-dict，或用户词表为空）");
        return Ok(());
    };
    let readings = DictReadings::load(args)?;
    let mut report = Report::default();
    for entry in user.entries() {
        classify(entry.text, entry.pinyin, &readings, &mut report);
    }
    print!("{}", render(&report));

    if args.purge_user_words {
        let rows: Vec<&Row> = report.mismatch.iter().chain(&report.suspect).collect();
        for row in &rows {
            engine.learner_mut().forget(&row.text);
        }
        engine.learner_mut().flush();
        if let Some(path) = &args.user_dict {
            verify_purged(path, &rows)?;
        }
        println!("# 已删除 {} 条错读音用户词并落盘", rows.len());
    }
    Ok(())
}

/// 用户词文件名，与词频文件同目录（`FrequencyLearner::words_path`）。
const WORDS_FILE: &str = "user-words.tsv";

/// 删完后读回 `user-words.tsv` 核实：这次要删的词必须已经不在文件里。
///
/// `flush` 写失败不返回错误，不能凭它没吭声就宣布「已落盘」；文件不存在按已删干净处理
/// （词全删完就是空文件，或压根没写过）。
fn verify_purged(path: &Path, rows: &[&Row]) -> Result<(), CliError> {
    let file = path.with_file_name(WORDS_FILE);
    let source = match std::fs::read_to_string(&file) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let remaining: Vec<&str> = rows
        .iter()
        .map(|row| row.text.as_str())
        .filter(|text| {
            source
                .lines()
                .any(|line| line.split('\t').next() == Some(*text))
        })
        .collect();
    if remaining.is_empty() {
        Ok(())
    } else {
        Err(CliError::UserWordsRemain(remaining.join("、")))
    }
}

/// 判定一个用户词落哪一档（都不落就跳过）。
fn classify(text: &str, reading: &str, readings: &DictReadings, report: &mut Report) {
    if let Some(known) = readings.words.get(text) {
        if !known.contains(reading) {
            report.mismatch.push(Row {
                text: text.to_owned(),
                reading: reading.to_owned(),
                detail: join_limited(known),
            });
        }
        return;
    }

    // 词库不认识这个词：逐字核对，口径与引擎的 accepts_reading 一致——词库不认识这个字（生僻字）时没法核对，放行
    let syllables: Vec<&str> = reading.split(' ').collect();
    let chars: Vec<char> = text.chars().collect();
    if chars.len() != syllables.len() {
        report.suspect.push(Row {
            text: text.to_owned(),
            reading: reading.to_owned(),
            detail: format!("字数 {} 与音节数 {} 不符", chars.len(), syllables.len()),
        });
        return;
    }
    let mut offenders: Vec<String> = Vec::new();
    for (ch, syllable) in chars.iter().zip(&syllables) {
        let Some(known) = readings.chars.get(ch) else {
            continue;
        };
        if !known.contains(*syllable) {
            offenders.push(format!("{ch} {syllable}（词库：{}）", join_limited(known)));
        }
    }
    if !offenders.is_empty() {
        report.suspect.push(Row {
            text: text.to_owned(),
            reading: reading.to_owned(),
            detail: offenders.join("；"),
        });
    }
}

/// 打印结果（只读与删除前都先看这份）。
fn render(report: &Report) -> String {
    let mut out = format!(
        "# 核对用户词：读音不符 {} 条、读音可疑 {} 条\n",
        report.mismatch.len(),
        report.suspect.len()
    );
    if report.mismatch.is_empty() && report.suspect.is_empty() {
        out.push_str("# 没有问题\n");
        return out;
    }
    if !report.mismatch.is_empty() {
        out.push_str("# 读音不符（词库有该词，存的读音不是词库给的读音）\n");
        out.push_str("词\t存的读音\t词库读音\n");
        for row in &report.mismatch {
            out.push_str(&format!("{}\t{}\t{}\n", row.text, row.reading, row.detail));
        }
    }
    if !report.suspect.is_empty() {
        out.push_str("# 读音可疑（词库不认识该词，某个字按存的读音不是这个字的读音）\n");
        out.push_str("词\t存的读音\t不合的字音\n");
        for row in &report.suspect {
            out.push_str(&format!("{}\t{}\t{}\n", row.text, row.reading, row.detail));
        }
    }
    out
}

/// 读音集合拼成一行，最多列 [`MAX_READINGS`] 条，多了省略。
fn join_limited(readings: &HashSet<String>) -> String {
    let mut items: Vec<&str> = readings.iter().map(String::as_str).collect();
    items.sort_unstable();
    let shown = items.iter().take(MAX_READINGS).copied().collect::<Vec<_>>();
    let mut joined = shown.join(" / ");
    if items.len() > MAX_READINGS {
        joined.push_str(" …");
    }
    joined
}

/// 一行里最多列几条读音。
const MAX_READINGS: usize = 6;

impl DictReadings {
    /// 从主词库（与 `--extra-dict`）建读音参照。
    fn load(args: &Args) -> Result<Self, CliError> {
        let path = args.dict.clone().unwrap_or_else(default_dict);
        let mut dictionaries = vec![Dictionary::from_path(path)?];
        for extra in &args.extra_dict {
            dictionaries.push(Dictionary::from_path(extra)?);
        }
        let mut readings = Self::default();
        for dictionary in &dictionaries {
            for entry in dictionary.entries() {
                readings
                    .words
                    .entry(entry.text.to_owned())
                    .or_default()
                    .insert(entry.pinyin.to_owned());
                let mut chars = entry.text.chars();
                if let (Some(ch), None) = (chars.next(), chars.next()) {
                    readings
                        .chars
                        .entry(ch)
                        .or_default()
                        .insert(entry.pinyin.to_owned());
                }
            }
        }
        Ok(readings)
    }
}

/// 与 `build_engine` 同一套缺省：`data/generated/dict.qj` 优先，退回仓库自带的样例。
fn default_dict() -> PathBuf {
    crate::args::default_data_file("dict.tsv")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn readings(words: &[(&str, &str)], chars: &[(&str, &str)]) -> DictReadings {
        let mut readings = DictReadings::default();
        for (text, pinyin) in words {
            readings
                .words
                .entry((*text).to_owned())
                .or_default()
                .insert((*pinyin).to_owned());
        }
        for (text, syllable) in chars {
            let ch = text.chars().next().unwrap();
            readings
                .chars
                .entry(ch)
                .or_default()
                .insert((*syllable).to_owned());
        }
        readings
    }

    #[test]
    fn flags_word_in_dictionary_with_a_different_reading() {
        // 词库「我的」是 wo de；用户词存的是 wo di → 读音不符
        let readings = readings(
            &[("我的", "wo de")],
            &[("我", "wo"), ("的", "de"), ("的", "di")],
        );
        let mut report = Report::default();
        classify("我的", "wo di", &readings, &mut report);
        assert_eq!(report.mismatch.len(), 1);
        assert!(report.suspect.is_empty());
        assert_eq!(report.mismatch[0].detail, "wo de");
    }

    #[test]
    fn leaves_correctly_read_words_alone() {
        let readings = readings(&[("我的", "wo de")], &[("的", "de")]);
        let mut report = Report::default();
        classify("我的", "wo de", &readings, &mut report);
        assert!(report.mismatch.is_empty() && report.suspect.is_empty());
    }

    #[test]
    fn flags_unknown_word_whose_char_reading_is_not_in_dictionary() {
        // 词库不认识这个词，逐字核对：呢 neng 不是词库里「呢」的读音 → 读音可疑
        let readings = readings(&[], &[("呢", "ne"), ("嗯", "en")]);
        let mut report = Report::default();
        classify("呢嗯", "neng en", &readings, &mut report);
        assert!(report.mismatch.is_empty());
        assert_eq!(report.suspect.len(), 1);
        assert_eq!(report.suspect[0].detail, "呢 neng（词库：ne）");
    }

    #[test]
    fn accepts_char_the_dictionary_never_knows() {
        // 词库压根不认识「囧」（生僻字），没法核对，放行
        let readings = readings(&[], &[("嗯", "en")]);
        let mut report = Report::default();
        classify("囧嗯", "jiong en", &readings, &mut report);
        assert!(report.mismatch.is_empty() && report.suspect.is_empty());
    }

    #[test]
    fn flags_arity_mismatch() {
        let readings = readings(&[], &[("的", "de")]);
        let mut report = Report::default();
        classify("的的", "de", &readings, &mut report);
        assert_eq!(report.suspect.len(), 1);
        assert_eq!(report.suspect[0].detail, "字数 2 与音节数 1 不符");
    }

    /// 落盘核实：文件里还留着要删的词就是没写进磁盘
    #[test]
    fn flags_words_left_on_disk() {
        let dir = std::env::temp_dir().join(format!("qingjian-purge-check-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let user_tsv = dir.join("user.tsv");
        let rows = [Row {
            text: "我的".to_owned(),
            reading: "wo di".to_owned(),
            detail: String::new(),
        }];
        let refs: Vec<&Row> = rows.iter().collect();

        std::fs::write(
            dir.join(WORDS_FILE),
            "开发\tkai fa\t100\n我的\two di\t100\n",
        )
        .unwrap();
        assert!(verify_purged(&user_tsv, &refs).is_err());

        std::fs::write(dir.join(WORDS_FILE), "开发\tkai fa\t100\n").unwrap();
        assert!(verify_purged(&user_tsv, &refs).is_ok());
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 文件不在（词全删完、或压根没写过）也算删干净
    #[test]
    fn treats_absent_words_file_as_purged() {
        let dir = std::env::temp_dir().join(format!("qingjian-purge-gone-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let _ = std::fs::remove_file(dir.join(WORDS_FILE));
        let row = Row {
            text: "我的".to_owned(),
            reading: "wo di".to_owned(),
            detail: String::new(),
        };
        assert!(verify_purged(&dir.join("user.tsv"), &[&row]).is_ok());
        std::fs::remove_dir_all(&dir).ok();
    }
}
