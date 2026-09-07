//! 从纯文本语料统计词级一元 / 二元计数。
//!
//! 分词用青简自己的词库做一元最大概率切分（与输入法词图同一套词表，统计出来的词才能在整句转换里用上）；
//! 只统计连续的汉字段，段与段之间（标点、数字、字母）算句子边界，句首用 `<s>` 标记；空格忽略（预分词语料）。
//! 词库里没有的字跳过，并切断前后的二元关系。

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::error::ConvertError;
use crate::oov_filter::OovFilter;

/// 句首标记。
const SENTENCE_START: &str = "<s>";

/// 分词时一个词最多几个汉字。
const MAX_WORD_CHARS: usize = 8;

/// 词库里没有的单字的 log 概率（相对于词频 1 的词再扣这么多）。
const UNKNOWN_PENALTY: f64 = -12.0;

/// 分词用的词表：词 → 编号与 log 词频。
struct Vocabulary {
    /// 词 → 编号。
    ids: HashMap<String, u32>,

    /// 编号 → 词。
    words: Vec<String>,

    /// 编号 → log 概率：log(词频 + 1) − log(总词频)。不减总频的话多字词会输给它的单字。
    log_frequency: Vec<f64>,

    /// 未知单字的 log 概率。
    unknown: f64,
}

impl Vocabulary {
    /// 读分词词表：`path` 加上同目录 `dicts/` 下的领域词库（拆分后基础词库不含领域词，分词仍要用全部词）。
    fn load(path: &Path) -> Result<Self, ConvertError> {
        let mut files = vec![path.to_path_buf()];
        if let Some(dir) = path.parent().map(|p| p.join("dicts"))
            && let Ok(entries) = std::fs::read_dir(&dir)
        {
            let mut extra: Vec<_> = entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "tsv"))
                .collect();
            extra.sort();
            tracing::info!(dir = %dir.display(), files = extra.len(), "分词也用领域词库");
            files.extend(extra);
        }
        let mut total = 0.0_f64;
        let mut ids: HashMap<String, u32> = HashMap::new();
        let mut words = vec![SENTENCE_START.to_owned()];
        let mut log_frequency = vec![0.0];
        ids.insert(SENTENCE_START.to_owned(), 0);
        for line in files
            .iter()
            .map(|file| File::open(file).map(BufReader::new))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flat_map(BufRead::lines)
        {
            let line = line?;
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let (Some(text), Some(_), Some(frequency)) =
                (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            let frequency: f64 = frequency.trim().parse().unwrap_or(0.0);
            total += frequency;
            let log = (frequency + 1.0).ln();
            match ids.get(text) {
                // 同一个词多个读音：取最高词频
                Some(&id) => {
                    if log > log_frequency[id as usize] {
                        log_frequency[id as usize] = log;
                    }
                }
                None => {
                    ids.insert(text.to_owned(), words.len() as u32);
                    words.push(text.to_owned());
                    log_frequency.push(log);
                }
            }
        }
        let log_total = total.max(1.0).ln();
        for log in &mut log_frequency {
            *log -= log_total;
        }
        Ok(Self {
            ids,
            words,
            log_frequency,
            unknown: UNKNOWN_PENALTY - log_total,
        })
    }

    /// 一段连续汉字按最大概率切成词编号；词库里没有的字用 `None` 占位。
    fn segment(&self, run: &str, output: &mut Vec<Option<u32>>) {
        output.clear();
        let offsets: Vec<usize> = run
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(run.len()))
            .collect();
        let n = offsets.len() - 1;
        let mut best = vec![f64::NEG_INFINITY; n + 1];
        let mut back: Vec<(usize, Option<u32>)> = vec![(0, None); n + 1];
        best[0] = 0.0;
        for start in 0..n {
            if best[start] == f64::NEG_INFINITY {
                continue;
            }
            let mut any = false;
            for end in start + 1..=n.min(start + MAX_WORD_CHARS) {
                let slice = &run[offsets[start]..offsets[end]];
                if let Some(&id) = self.ids.get(slice) {
                    any = true;
                    let score = best[start] + self.log_frequency[id as usize];
                    if score > best[end] {
                        best[end] = score;
                        back[end] = (start, Some(id));
                    }
                }
            }
            if !any {
                let score = best[start] + self.unknown;
                if score > best[start + 1] {
                    best[start + 1] = score;
                    back[start + 1] = (start, None);
                }
            }
        }
        let mut position = n;
        while position > 0 {
            let (start, id) = back[position];
            output.push(id);
            position = start;
        }
        output.reverse();
    }
}

fn is_han(c: char) -> bool {
    matches!(c, '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}')
}

/// `mine` 的参数。
pub struct MineOptions {
    /// 语料文件。
    pub corpus: Vec<PathBuf>,

    /// 分词用的词库。
    pub dict: PathBuf,

    /// 次数下限。
    pub min_count: u32,

    /// 最多几个字。
    pub max_chars: usize,

    /// 一元表（算 PMI 用）。
    pub frequency: PathBuf,

    /// PMI 下限，0 不过滤。
    pub min_pmi: f64,

    /// 只过滤这份现成的候选文件，不扫语料。
    pub candidates: Option<PathBuf>,
}

/// 挖词库里没有的词：分词时连续落成单字的那一段（2–`max_chars` 个字）多半是一个词库没收的词，
/// 按出现次数统计，出现 `min_count` 次以上的写到 `oov-candidates.tsv`（`词\t次数`），
/// 再经 [`OovFilter`]（虚词规则 + 相邻字对 PMI）筛成 `oov-filtered.tsv` 与一行一词的 `oov-words.txt`，
/// 后者交给 `gloss-gen pinyin` 标音、前者给 `lexicon --extra-words` 并进词库。
/// 单字词本身在词库里（规范字全收了），所以这里只看「本可以成词却被拆成单字」的连续段：
/// 段内每个字都是词库里的单字词、且整段不在词库里。
pub fn mine(options: &MineOptions, out_dir: &Path) -> Result<(), ConvertError> {
    std::fs::create_dir_all(out_dir)?;
    let counts = match &options.candidates {
        Some(path) => read_candidates(path)?,
        None => {
            let counts = scan_single_runs(options)?;
            write_counts(
                &out_dir.join("oov-candidates.tsv"),
                "# 由 qingjian-dict-convert mine 从语料挖出的词库未收词（未过滤）。词\t次数",
                &sorted_rows(&counts, options.min_count),
            )?;
            counts
        }
    };
    let filter = OovFilter::from_unigram_file(&options.frequency, options.min_pmi)?;
    let kept: Vec<(String, u32)> = sorted_rows(&counts, options.min_count)
        .into_iter()
        .filter(|(word, _)| {
            if options.min_pmi <= 0.0 {
                OovFilter::passes_function_rules(word)
            } else {
                filter.keeps(word, &counts)
            }
        })
        .collect();
    write_counts(
        &out_dir.join("oov-filtered.tsv"),
        &format!(
            "# mine 结果经虚词规则 + 相邻字对 PMI≥{} 过滤。词\t次数",
            options.min_pmi
        ),
        &kept,
    )?;
    let words_path = out_dir.join("oov-words.txt");
    let mut writer = BufWriter::new(File::create(&words_path)?);
    for (word, _) in &kept {
        writeln!(writer, "{word}")?;
    }
    writer.flush()?;
    tracing::info!(
        candidates = counts.values().filter(|c| **c >= options.min_count).count(),
        kept = kept.len(),
        path = %words_path.display(),
        "挖词过滤完成"
    );
    Ok(())
}

/// 读上一次写出的候选文件（`词\t次数`，`#` 注释）。
fn read_candidates(path: &Path) -> Result<HashMap<String, u32>, ConvertError> {
    let mut counts = HashMap::new();
    for line in BufReader::new(File::open(path)?).lines() {
        let line = line?;
        if line.starts_with('#') {
            continue;
        }
        if let Some((word, count)) = line.split_once('\t')
            && let Ok(count) = count.parse::<u32>()
        {
            counts.insert(word.to_owned(), count);
        }
    }
    Ok(counts)
}

/// 次数不低于 `min_count` 的候选，按次数降序、同次数按词排。
fn sorted_rows(counts: &HashMap<String, u32>, min_count: u32) -> Vec<(String, u32)> {
    let mut rows: Vec<(String, u32)> = counts
        .iter()
        .filter(|(_, c)| **c >= min_count)
        .map(|(w, c)| (w.clone(), *c))
        .collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    rows
}

fn write_counts(path: &Path, header: &str, rows: &[(String, u32)]) -> Result<(), ConvertError> {
    let mut writer = BufWriter::new(File::create(path)?);
    writeln!(writer, "{header}")?;
    for (word, count) in rows {
        writeln!(writer, "{word}\t{count}")?;
    }
    writer.flush()?;
    tracing::info!(rows = rows.len(), path = %path.display(), "已写出");
    Ok(())
}

/// 扫语料，数连续单字段里 2–`max_chars` 字的子串。
fn scan_single_runs(options: &MineOptions) -> Result<HashMap<String, u32>, ConvertError> {
    let MineOptions {
        corpus,
        dict,
        max_chars,
        ..
    } = options;
    let max_chars = *max_chars;
    let vocabulary = Vocabulary::load(dict)?;
    tracing::info!(words = vocabulary.words.len(), "词表加载完成");
    let single_ids: std::collections::HashSet<u32> = vocabulary
        .words
        .iter()
        .enumerate()
        .filter(|(_, w)| w.chars().count() == 1)
        .map(|(i, _)| i as u32)
        .collect();
    let mut counts: HashMap<String, u32> = HashMap::new();
    let mut tokens = Vec::new();
    let mut lines = 0u64;
    for path in corpus {
        tracing::info!(path = %path.display(), "扫描语料");
        for line in BufReader::new(File::open(path)?).lines() {
            let line = line?;
            lines += 1;
            if lines.is_multiple_of(500_000) {
                tracing::info!(lines, candidates = counts.len(), "进度");
                // 内存兜底：候选太多先把只出现一次的扔掉
                if counts.len() > 20_000_000 {
                    counts.retain(|_, c| *c > 1);
                }
            }
            let line: String = line.chars().filter(|c| *c != ' ').collect();
            for run in line.split(|c: char| !is_han(c)) {
                if run.chars().count() < 2 {
                    continue;
                }
                vocabulary.segment(run, &mut tokens);
                // 连续单字段：token 是单字词编号的一串
                let mut start = 0usize;
                let chars: Vec<char> = run.chars().collect();
                let mut position = 0usize;
                let mut singles: Vec<usize> = Vec::new();
                for token in &tokens {
                    let width = match token {
                        Some(id) => vocabulary.words[*id as usize].chars().count(),
                        None => 1,
                    };
                    let is_single = token.is_some_and(|id| single_ids.contains(&id));
                    if is_single {
                        if singles.is_empty() {
                            start = position;
                        }
                        singles.push(position);
                    } else if !singles.is_empty() {
                        collect_runs(&chars, start, position, max_chars, &mut counts);
                        singles.clear();
                    }
                    position += width;
                }
                if !singles.is_empty() {
                    collect_runs(&chars, start, position, max_chars, &mut counts);
                }
            }
        }
    }
    tracing::info!(lines, candidates = counts.len(), "扫描完成");
    Ok(counts)
}

/// 一段连续单字 `chars[start..end]` 里所有 2–`max_chars` 字的子串各计一次。
fn collect_runs(
    chars: &[char],
    start: usize,
    end: usize,
    max_chars: usize,
    counts: &mut HashMap<String, u32>,
) {
    let len = end - start;
    if len < 2 {
        return;
    }
    for width in 2..=max_chars.min(len) {
        for from in start..=end - width {
            let word: String = chars[from..from + width].iter().collect();
            *counts.entry(word).or_insert(0) += 1;
        }
    }
}

pub fn convert(
    corpus: &[PathBuf],
    dict: &Path,
    min_count: u32,
    max_bigrams: usize,
    out_dir: &Path,
) -> Result<(), ConvertError> {
    let vocabulary = Vocabulary::load(dict)?;
    tracing::info!(words = vocabulary.words.len(), "词表加载完成");
    let mut unigram: Vec<u64> = vec![0; vocabulary.words.len()];
    let mut bigram: HashMap<u64, u32> = HashMap::new();
    let mut tokens = Vec::new();
    let mut lines = 0u64;
    let mut runs = 0u64;
    for path in corpus {
        tracing::info!(path = %path.display(), "统计语料");
        for line in BufReader::new(File::open(path)?).lines() {
            let line = line?;
            lines += 1;
            if lines.is_multiple_of(200_000) {
                tracing::info!(lines, bigrams = bigram.len(), "进度");
            }
            // LCCC 这类语料是按词用空格分好的；空格不是句子边界，去掉再切
            let line: String = line.chars().filter(|c| *c != ' ').collect();
            for run in line.split(|c: char| !is_han(c)) {
                if run.chars().count() < 2 {
                    continue;
                }
                runs += 1;
                vocabulary.segment(run, &mut tokens);
                unigram[0] += 1;
                let mut previous: Option<u32> = Some(0);
                for token in &tokens {
                    match token {
                        Some(id) => {
                            unigram[*id as usize] += 1;
                            if let Some(prev) = previous {
                                *bigram
                                    .entry((u64::from(prev) << 32) | u64::from(*id))
                                    .or_insert(0) += 1;
                            }
                            previous = Some(*id);
                        }
                        None => previous = None,
                    }
                }
            }
        }
    }
    tracing::info!(
        lines,
        sentences = runs,
        distinct_bigrams = bigram.len(),
        "统计完成"
    );

    // 二元：按计数降序，砍掉低频与超出上限的
    let mut pairs: Vec<(u64, u32)> = bigram
        .into_iter()
        .filter(|(_, count)| *count >= min_count)
        .collect();
    pairs.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    pairs.truncate(max_bigrams);
    let bigram_path = out_dir.join("lm-bigram.tsv");
    let mut writer = BufWriter::new(File::create(&bigram_path)?);
    writeln!(
        writer,
        "# 由 qingjian-dict-convert bigram 从语料统计。前词\\t后词\\t计数"
    )?;
    for (key, count) in &pairs {
        let first = &vocabulary.words[(key >> 32) as usize];
        let second = &vocabulary.words[(key & 0xFFFF_FFFF) as usize];
        writeln!(writer, "{first}\t{second}\t{count}")?;
    }
    writer.flush()?;

    // 一元：只输出出现过的词
    let unigram_path = out_dir.join("lm-unigram.tsv");
    let mut writer = BufWriter::new(File::create(&unigram_path)?);
    writeln!(
        writer,
        "# 由 qingjian-dict-convert bigram 从语料统计。词\\t计数；<s> 是句首标记"
    )?;
    let mut written = 0usize;
    for (id, count) in unigram.iter().enumerate() {
        if *count > 0 {
            writeln!(writer, "{}\t{count}", vocabulary.words[id])?;
            written += 1;
        }
    }
    writer.flush()?;
    tracing::info!(
        unigram = %unigram_path.display(),
        words = written,
        bigram = %bigram_path.display(),
        bigrams = pairs.len(),
        "写出完成"
    );
    Ok(())
}
