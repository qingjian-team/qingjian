//! 短语层：常用词表是词典词头，不收 我的 / 好的 / 不知道 / 有没有 这类人会整块打出来的组合，输入法里它们却是最常打的。
//! 扫两遍语料：第一遍按词库分词数相邻两词，第二遍只数两段二元都够频的相邻三词；取出现够多、边界像话的组合，读音直接由成分词的读音拼出（不用再标多音字），写成 `phrases.tsv`（`词\t次数\t拼音`），
//! 由 `lexicon --extra-words` 并入基础词库。
//!
//! 次数按语料文件分开数，短语要在**对话语料**（`--dialogue`，人打字的样子）里就够频：维基里的模板句（一个市镇、面积为）
//! 次数再高也不是人会打的。只按次数与边界规则（[`rules`]）筛，不用点互信息：虚词组合的 PMI 天然低，而它们正是要收的。
//! 结果要人工过一遍再拷进 `assets/lexicon/`。
//!
//! 分词用的词库必须是**还没并入短语**的：并入后 我的 是一个词，相邻词对里就没有 我 + 的 了。重跑时给 `--refresh <上次的 phrases.tsv>`，
//! 里面的词会先从分词词表里摘掉。

mod rules;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::bigram::{Vocabulary, is_han};
use crate::error::ConvertError;

/// `phrases` 子命令的参数。
pub struct PhraseOptions {
    /// 分词与成分读音用的词库（同目录 `dicts/` 一并读）。
    pub dict: PathBuf,

    /// 上一次的短语表：里面的词先从分词词表里摘掉（词库已并入短语时重跑用）。
    pub refresh: Option<PathBuf>,

    /// 语料。
    pub corpus: Vec<PathBuf>,

    /// 对话语料（`corpus` 里的一个）：短语在它里面的次数也要够 `min_count`。
    pub dialogue: PathBuf,

    /// 次数下限。
    pub min_count: u32,

    /// 短语最多几个字。
    pub max_chars: usize,
}

/// 次数超过下限这么多倍的算「够常见」，形状规则不再筛它（见 [`rules::passes`]）。
const STRONG_MULTIPLIER: u32 = 5;

/// 一条短语候选。
struct Phrase {
    text: String,
    count: u32,
    pinyin: String,
}

pub fn mine(options: &PhraseOptions, out_dir: &Path) -> Result<(), ConvertError> {
    let mut vocabulary = Vocabulary::load(&options.dict)?;
    let readings = load_readings(&options.dict)?;
    if let Some(path) = &options.refresh {
        let previous = read_words(path)?;
        let before = vocabulary.ids.len();
        vocabulary.ids.retain(|word, _| !previous.contains(word));
        tracing::info!(
            path = %path.display(),
            removed = before - vocabulary.ids.len(),
            "上次的短语已从分词词表摘掉"
        );
    }
    tracing::info!(
        words = vocabulary.ids.len(),
        readings = readings.len(),
        "词表加载完成"
    );
    let known: HashSet<&str> = vocabulary.ids.keys().map(String::as_str).collect();
    let pairs = count(&options.corpus, &options.dialogue, &vocabulary, None)?;
    let frequent_pairs: HashSet<(u32, u32)> = pairs
        .iter()
        .filter(|(_, t)| t.total >= options.min_count)
        .map(|(ids, _)| (ids[0], ids[1]))
        .collect();
    tracing::info!(pairs = frequent_pairs.len(), "够频的二元（三词短语的预筛）");
    let triples = count(
        &options.corpus,
        &options.dialogue,
        &vocabulary,
        Some(&frequent_pairs),
    )?;
    let counts = pairs.into_iter().chain(triples);

    let mut phrases: Vec<Phrase> = Vec::new();
    for (ids, tally) in counts {
        if tally.total < options.min_count || tally.dialogue < options.min_count {
            continue;
        }
        let parts: Vec<&str> = ids
            .iter()
            .map(|id| vocabulary.words[*id as usize].as_str())
            .collect();
        let text = parts.concat();
        let chars = text.chars().count();
        if !(2..=options.max_chars).contains(&chars)
            || known.contains(text.as_str())
            || !rules::passes(&parts, tally.total >= STRONG_MULTIPLIER * options.min_count)
        {
            continue;
        }
        let syllables: Option<Vec<&str>> = parts
            .iter()
            .map(|p| readings.get(*p).map(String::as_str))
            .collect();
        let Some(syllables) = syllables else {
            continue;
        };
        phrases.push(Phrase {
            text,
            count: tally.total,
            pinyin: syllables.join(" "),
        });
    }
    phrases.sort_by(|x, y| y.count.cmp(&x.count).then_with(|| x.text.cmp(&y.text)));
    phrases.dedup_by(|x, y| x.text == y.text);
    let path = out_dir.join("phrases.tsv");
    let mut writer = BufWriter::new(File::create(&path)?);
    writeln!(
        writer,
        "# 由 qingjian-dict-convert phrases 从语料相邻词组合挖出（总次数与对话语料次数都 ≥ {}），读音由成分词拼出。词\\t次数\\t拼音",
        options.min_count
    )?;
    for phrase in &phrases {
        writeln!(
            writer,
            "{}\t{}\t{}",
            phrase.text, phrase.count, phrase.pinyin
        )?;
    }
    writer.flush()?;
    tracing::info!(path = %path.display(), phrases = phrases.len(), "写出完成");
    Ok(())
}

/// 一个词组合在全部语料与对话语料里各出现几次。
#[derive(Debug, Default, Clone, Copy)]
struct Tally {
    /// 全部语料。
    total: u32,

    /// 对话语料。
    dialogue: u32,
}

/// 读一个 `词\t…` 文件的第一列。
fn read_words(path: &Path) -> Result<HashSet<String>, ConvertError> {
    Ok(std::fs::read_to_string(path)?
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split('\t').next().map(str::to_owned))
        .collect())
}

/// 扫语料数相邻词组合：`frequent_pairs` 为 `None` 数全部相邻两词，否则数两段二元都在里面的相邻三词。
fn count(
    corpus: &[PathBuf],
    dialogue: &Path,
    vocabulary: &Vocabulary,
    frequent_pairs: Option<&HashSet<(u32, u32)>>,
) -> Result<HashMap<Vec<u32>, Tally>, ConvertError> {
    let mut counts: HashMap<Vec<u32>, Tally> = HashMap::new();
    let mut tokens = Vec::new();
    let mut lines = 0u64;
    for path in corpus {
        let is_dialogue = path == dialogue;
        tracing::info!(path = %path.display(), dialogue = is_dialogue, "扫描语料");
        let mut bump = |ids: &[u32]| {
            let tally = counts.entry(ids.to_vec()).or_default();
            tally.total += 1;
            if is_dialogue {
                tally.dialogue += 1;
            }
        };
        for line in BufReader::new(File::open(path)?).lines() {
            let line = line?;
            lines += 1;
            if lines.is_multiple_of(500_000) {
                tracing::info!(lines, "进度");
            }
            let line: String = line.chars().filter(|c| *c != ' ').collect();
            for run in line.split(|c: char| !is_han(c)) {
                if run.chars().count() < 2 {
                    continue;
                }
                vocabulary.segment(run, &mut tokens);
                // 笑声串（哈哈哈 切成 哈哈 + 哈）不算相邻：你哈哈哈 会把 你 + 哈 灌成高频短语；孤立的 哈（对哈）照常
                let laughter: Vec<bool> = tokens
                    .iter()
                    .map(|t| t.is_some_and(|id| vocabulary.words[id as usize].starts_with('哈')))
                    .collect();
                let in_run = |i: usize| {
                    laughter[i]
                        && ((i > 0 && laughter[i - 1])
                            || (i + 1 < laughter.len() && laughter[i + 1]))
                };
                match frequent_pairs {
                    None => {
                        for (i, window) in tokens.windows(2).enumerate() {
                            if let (Some(a), Some(b)) = (window[0], window[1])
                                && !in_run(i)
                                && !in_run(i + 1)
                            {
                                bump(&[a, b]);
                            }
                        }
                    }
                    Some(pairs) => {
                        for (i, window) in tokens.windows(3).enumerate() {
                            if let (Some(a), Some(b), Some(c)) = (window[0], window[1], window[2])
                                && pairs.contains(&(a, b))
                                && pairs.contains(&(b, c))
                                && !(in_run(i) || in_run(i + 1) || in_run(i + 2))
                            {
                                bump(&[a, b, c]);
                            }
                        }
                    }
                }
            }
        }
        // 内存兜底：只出现一次的二元扔掉
        counts.retain(|_, t| t.total > 1);
        tracing::info!(lines, combinations = counts.len(), "本文件扫描完成");
    }
    Ok(counts)
}

/// 词 → 读音（同一个词多个读音取词频最高的那条），读 `dict` 与同目录 `dicts/`。
fn load_readings(dict: &Path) -> Result<HashMap<String, String>, ConvertError> {
    let mut best: HashMap<String, (u64, String)> = HashMap::new();
    for path in Vocabulary::files(dict) {
        for line in BufReader::new(File::open(&path)?).lines() {
            let line = line?;
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let (Some(text), Some(pinyin), Some(frequency)) =
                (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            let frequency: u64 = frequency.trim().parse().unwrap_or(0);
            match best.get_mut(text) {
                Some(entry) if entry.0 >= frequency => {}
                Some(entry) => *entry = (frequency, pinyin.to_owned()),
                None => {
                    best.insert(text.to_owned(), (frequency, pinyin.to_owned()));
                }
            }
        }
    }
    Ok(best.into_iter().map(|(t, (_, p))| (t, p)).collect())
}
