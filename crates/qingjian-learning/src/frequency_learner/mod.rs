mod learner_impl;
mod tables;

use std::collections::BTreeMap;

use foldhash::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use qingjian_core::sentence::{Context, UserNgram};
use qingjian_core::storage::{read_text_lossy, write_atomic};
use qingjian_core::{Candidate, Forgotten, Learner};
use qingjian_dictionary::{Dictionary, WordList};

use crate::error::LearningError;

/// 用户词文件名，与词频文件放同一目录；格式与主词库 TSV 相同（`词\t拼音\t词频`）。
const USER_WORDS_FILE: &str = "user-words.tsv";

/// 用户词在小词库里的词频。排序主要靠 weight，这个值只在同 weight 时起作用。
const USER_WORD_FREQUENCY: u32 = 100;

/// 个人 n-gram 文件名，与词频文件同目录：`前词\t后词\t次数`，句首用 `<s>`。
const USER_NGRAM_FILE: &str = "user-ngram.tsv";

/// 按输入串记的选择文件名，与词频文件同目录：`输入串\t词\t次数`。
const USER_CHOICES_FILE: &str = "user-choices.tsv";

/// 个人英文词表文件名，与词频文件同目录：`词\t次数`（词按第一次敲的写法存）。
const USER_ENGLISH_FILE: &str = "user-english.tsv";

/// 个人敲错表文件名，与词频文件同目录：`敲的\t要的\t次数`（音节级，接受过的纠正）。
const USER_TYPOS_FILE: &str = "user-typos.tsv";

/// 按输入串记的选择最多存多少条 (输入串, 词)，超过就整体减半（忘掉久远的偏好）。
const MAX_CHOICE_ENTRIES: usize = 50_000;

/// 「回车原样上屏过」在选择表里的记法：词那一列写这个标记（尖括号不可能是候选词）。
const RAW_MARK: &str = "<raw>";

/// 内存中的用户词频表。
#[derive(Debug, Default)]
pub struct FrequencyLearner {
    /// 词 → 用户选择次数。
    counts: HashMap<String, u32>,

    /// 自上次保存后是否有新记录。
    dirty: bool,

    /// [`Learner::flush`] 时写回的路径；`None` 表示只在内存里学习。
    path: Option<PathBuf>,

    /// 用户词：词 → 全拼（空格分隔）。词库里没有、用户却选过的词（云联想接受的等）。
    words: BTreeMap<String, String>,

    /// 用户词组成的小词库，随 `words` 重建。
    user_dictionary: Option<Dictionary>,

    /// 用户词自上次保存后是否有变化。
    words_dirty: bool,

    /// 个人 n-gram：上屏词序列的转移计数（二元 + 三元）。
    ngram: UserNgram,

    /// 个人 n-gram 自上次保存后是否有变化。
    ngram_dirty: bool,

    /// 输入串 → (词 → 在这个输入串下被选的次数)。
    choices: HashMap<String, HashMap<String, u32>>,

    /// 按输入串记的选择自上次保存后是否有变化。
    choices_dirty: bool,

    /// 个人英文词：小写编码 → (第一次敲的写法, 次数)。回车原样上屏的英文词、选过的英文候选。
    english: BTreeMap<String, (String, u32)>,

    /// 个人英文词组成的词表，随 `english` 重建。
    english_list: Option<WordList>,

    /// 个人英文词自上次保存后是否有变化。
    english_dirty: bool,

    /// 个人敲错表：敲的那段字母 → (要的音节 → 接受次数)。
    typos: HashMap<String, HashMap<String, u32>>,

    /// 个人敲错表自上次保存后是否有变化。
    typos_dirty: bool,
}

impl FrequencyLearner {
    /// 从 `词\t次数` 文本建表（只在内存里学习）。坏行跳过。
    pub fn parse(source: &str) -> Self {
        let mut learner = Self::default();
        learner.load_counts(source);
        learner
    }

    /// 从 `词\t次数` 读选择次数，返回跳过的坏行数。
    fn load_counts(&mut self, source: &str) -> usize {
        let mut skipped = 0;
        for line in data_lines(source) {
            match line
                .split_once('\t')
                .and_then(|(text, count)| Some((text, count.trim().parse::<u32>().ok()?)))
            {
                Some((text, count)) => {
                    self.counts.insert(text.to_owned(), count);
                }
                None => skipped += 1,
            }
        }
        skipped
    }

    /// 从文件加载，之后 [`Learner::flush`] 会写回同一个文件。
    /// 文件不存在时返回空表，而不是报错：首次运行没有用户数据是正常的。
    ///
    /// 各文件按行容错：格式不对的行（崩溃写坏、手改错了）记一条警告跳过，其余照读，下次落盘时就没了；
    /// 编码坏掉的字节按替换字符读进来交给按行解析处理。只有真正的 io 错误（权限、坏盘）才返回 `Err`，
    /// 这时壳该退回只在内存里学习，别拿空表覆盖用户的文件。
    pub fn from_path(path: impl Into<PathBuf>) -> Result<Self, LearningError> {
        let path = path.into();
        let mut learner = Self::default();
        if let Some(source) = read_text_lossy(&path)? {
            let skipped = learner.load_counts(&source);
            note_skipped(&path, skipped);
        }
        let words_path = Self::words_path(&path);
        if let Some(source) = read_text_lossy(&words_path)? {
            let skipped = learner.load_words(&source);
            note_skipped(&words_path, skipped);
        }
        let ngram_path = Self::ngram_path(&path);
        if let Some(source) = read_text_lossy(&ngram_path)? {
            let (ngram, skipped) = UserNgram::parse_lenient(&source);
            learner.ngram = ngram;
            note_skipped(&ngram_path, skipped.len());
        }
        let choices_path = Self::choices_path(&path);
        if let Some(source) = read_text_lossy(&choices_path)? {
            let skipped = learner.load_choices(&source);
            note_skipped(&choices_path, skipped);
        }
        let english_path = Self::english_path(&path);
        if let Some(source) = read_text_lossy(&english_path)? {
            let skipped = learner.load_english(&source);
            note_skipped(&english_path, skipped);
        }
        let typos_path = Self::typos_path(&path);
        if let Some(source) = read_text_lossy(&typos_path)? {
            let skipped = learner.load_typos(&source);
            note_skipped(&typos_path, skipped);
        }
        learner.path = Some(path);
        Ok(learner)
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn len(&self) -> usize {
        self.counts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }

    pub fn save_to(&mut self, path: impl AsRef<Path>) -> Result<(), LearningError> {
        let mut rows: Vec<(&String, &u32)> = self.counts.iter().collect();
        rows.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        write_atomic(path.as_ref(), |file| {
            writeln!(file, "# 青简用户词频：词\\t选择次数")?;
            for (text, count) in rows {
                writeln!(file, "{text}\t{count}")?;
            }
            Ok(())
        })?;
        self.dirty = false;
        Ok(())
    }

    /// 有没有还没落盘的学习数据（任何一张表）。
    pub fn has_unsaved(&self) -> bool {
        self.dirty
            || self.words_dirty
            || self.english_dirty
            || self.ngram_dirty
            || self.choices_dirty
            || self.typos_dirty
    }
}

/// 数据行：去掉首尾空白、空行与 `#` 注释。
fn data_lines(source: &str) -> impl Iterator<Item = &str> {
    source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

/// 解析 `甲\t乙\t次数` 一行，格式不对返回 `None`。
fn parse_counted_pair(line: &str) -> Option<(&str, &str, u32)> {
    let mut fields = line.split('\t');
    let (Some(first), Some(second), Some(count)) = (fields.next(), fields.next(), fields.next())
    else {
        return None;
    };
    let count = count.trim().parse::<u32>().ok()?;
    Some((first, second, count))
}

/// 加载时跳过了坏行就记一条警告：用户能从日志里知道文件被写坏过，下次落盘会把坏行清掉。
fn note_skipped(path: &Path, skipped: usize) {
    if skipped > 0 {
        tracing::warn!(path = %path.display(), skipped, "学习数据里有格式不对的行，已跳过");
    }
}

#[cfg(test)]
mod tests;
