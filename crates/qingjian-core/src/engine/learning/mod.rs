//! 学习与统计的挂钩：释义兜底回填、词汇曝光、输入统计、输入日志、删候选、定时落盘。

use super::*;

mod forgotten;
mod learner;

pub use forgotten::Forgotten;
pub use learner::{Learner, NoLearner};

impl Engine {
    /// 取回释义兜底写好的释义，记进译者（个人释义表）；返回学了几条。壳定时调，不阻塞。
    /// 结果的语言与当前学习语言对不上（中途切过语言）就丢。
    pub fn poll_glosses(&mut self) -> usize {
        let filled = self.gloss_filler.poll();
        let mut learned = 0;
        for gloss in filled {
            if gloss.translation.language == self.translator.language() {
                tracing::debug!(word = %gloss.word, "释义兜底写入个人释义表");
                self.translator.learn(&gloss.word, gloss.translation);
                learned += 1;
            }
        }
        learned
    }

    /// 当前学习语言的词汇汇总（偏好设置「统计」页）。
    pub fn vocabulary_summary(&self) -> VocabularySummary {
        self.vocabulary.summary(self.translator.language())
    }

    /// 壳画完候选窗口后告知当前页上的候选：页上的译词在用户上屏那一刻记成「看到过」（[`VocabularyTracker`]）。
    /// 每次重画都换掉上一页，逐键刷新时一闪而过的候选不算；窗口收起时传空。
    pub fn note_displayed<'a>(&mut self, candidates: impl IntoIterator<Item = &'a Candidate>) {
        self.displayed.clear();
        for candidate in candidates {
            if candidate.kind == CandidateKind::English {
                continue;
            }
            let Some(translation) = &candidate.translation else {
                continue;
            };
            for sense in translation.senses() {
                let key = (translation.language, sense.text.clone());
                if !self.displayed.contains(&key) {
                    self.displayed.push(key);
                }
            }
        }
    }

    /// 上屏了：当前页上的译词都算看到过一轮。
    pub(super) fn record_exposures(&mut self) {
        for (language, word) in std::mem::take(&mut self.displayed) {
            self.vocabulary.record_exposure(language, &word);
        }
    }

    /// 按词汇记录给译词标生词：看到的轮次不到 [`FRESH_UNTIL`] 的算。
    pub(super) fn mark_fresh(&self, translation: &mut Translation) {
        let language = translation.language;
        for sense in translation.senses_mut() {
            sense.fresh = self.vocabulary.exposures(language, &sense.text) < FRESH_UNTIL;
        }
    }

    /// 往输入统计记一次上屏：汉字与英文词按文字数，中文词数按来源定（选一个词算一个，整句按语言模型切出来的词数，
    /// 切不了就按字数）。`english_word` 是原样上屏的字母串算不算一个英文词（拼音回车不算）。
    pub(super) fn meter_commit(&mut self, text: &str, source: InputSource, english_word: bool) {
        let mut usage = Usage::of_text(text);
        usage.words = match source {
            InputSource::Word | InputSource::Cloud => 1,
            InputSource::Sentence | InputSource::CloudSentence => {
                sentence::segment_text(text, &*self.language_model).map_or(usage.hanzi, |clauses| {
                    clauses.iter().map(|words| words.len() as u64).sum()
                })
            }
            InputSource::English
            | InputSource::Shortcut
            | InputSource::Emoji
            | InputSource::Raw
            | InputSource::Translation => 0,
        };
        if source == InputSource::Raw && !english_word {
            usage.english_words = 0;
        }
        self.meter.record(usage);
    }

    /// 往输入日志记一次上屏。`keys` 是这次消耗掉的原始键，`index` 从上一次查询的候选里找。
    pub(super) fn log_commit(&mut self, keys: &str, text: &str, source: InputSource) -> u64 {
        self.record_exposures();
        self.log_sequence += 1;
        let snapshot = self.last_query.borrow().clone().unwrap_or_default();
        let index = snapshot.candidates.iter().position(|c| c == text);
        self.logger.record(InputLogEntry::Commit(CommitEntry {
            id: self.log_sequence,
            scope: snapshot.scope,
            keys: keys.to_owned(),
            pinyin: snapshot.pinyin,
            corrected: snapshot.corrected,
            text: text.to_owned(),
            source,
            index,
            top: snapshot
                .candidates
                .into_iter()
                .take(LOGGED_CANDIDATES)
                .collect(),
            scheme: self
                .shuangpin
                .map_or_else(String::new, |s| s.key().to_owned()),
            english: self.english_mode,
        }));
        self.log_sequence
    }

    /// 用户要求删掉一个候选（修饰键 + 数字）：中文词与云端词交给 Learner 删用户词、清学习；英文词删个人英文词；
    /// 整句、快捷候选、emoji 没什么可删。删完缓存作废，它也不再当下一个词的上文。
    pub fn forget(&mut self, candidate: &Candidate) -> Forgotten {
        let forgotten = match candidate.kind {
            CandidateKind::Chinese | CandidateKind::Cloud => self.learner.forget(&candidate.text),
            CandidateKind::English => Forgotten {
                user_word: self.learner.forget_english(&candidate.text),
                learning: false,
            },
            CandidateKind::Sentence | CandidateKind::Shortcut | CandidateKind::Emoji => {
                Forgotten::default()
            }
        };
        if !forgotten.is_nothing() {
            self.forget_span_cache();
            *self.correction_cache.borrow_mut() = None;
            if self.chain.mentions(&candidate.text) {
                self.chain.reset();
            }
            self.recent_commits.retain(|c| c.text != candidate.text);
            tracing::debug!(text = %candidate.text, ?forgotten, "删除候选");
        }
        forgotten
    }

    /// 把学习数据与输入日志落盘。壳在停用输入法时调，激活期间也可以定时调（进程被杀时少丢）：
    /// 落盘不改学习状态，所以不像 [`Self::learner_mut`] 那样作废格子缓存。
    pub fn flush_learning(&mut self) {
        self.learner.flush();
        self.logger.flush();
        self.meter.flush();
        self.vocabulary.flush();
        self.translator.flush();
    }

    /// 词库 / 用户词 / 学习数据变了，整句格子候选全部作废。
    pub(super) fn forget_span_cache(&self) {
        self.span_cache.borrow_mut().clear();
    }
}
