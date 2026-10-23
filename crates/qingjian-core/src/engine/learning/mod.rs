//! 学习与统计的挂钩：释义兜底回填、词汇曝光、输入统计、输入日志、删候选、定时落盘。

use std::time::Instant;

use super::Engine;
use super::input_log::{CommitEntry, InputLogEntry, InputLogger, InputSource, LOGGED_CANDIDATES};
use super::statistics::Usage;
use super::vocabulary::{Encounter, FRESH_UNTIL, VocabularySummary};
use crate::candidate::{Candidate, CandidateKind, Language, Translation};
use crate::sentence;

mod forgotten;
mod learner;
mod muted;

pub use forgotten::Forgotten;
pub use learner::{Learner, NoLearner};
pub(super) use muted::MutedLearner;

impl Engine {
    /// 取回释义兜底写好的释义，记进译者（个人释义表）；返回学了几条。壳定时调，不阻塞。
    /// 结果的语言与当前学习语言对不上（中途切过语言）就丢。
    pub fn poll_glosses(&mut self) -> usize {
        let filled = self.gloss_filler.poll();
        let mut learned = 0;
        for gloss in filled {
            if gloss.translation.language == self.translator.language() {
                tracing::debug!(word = %gloss.word, "释义兜底写入个人释义表");
                // 兜底只为刚上屏的词问：它也算一次相遇
                self.record_encounter(&gloss.word, &gloss.translation, 0);
                self.translator.learn(&gloss.word, gloss.translation);
                learned += 1;
            }
        }
        for encounter in self.gloss_filler.poll_translations() {
            if encounter.language == self.translator.language() {
                self.vocabulary.record_encounter(&encounter);
            }
        }
        learned
    }

    /// 上屏了带译词的中文：记一次相遇，说法取直接打出的那条（没有就第一条）。单字不记，满屏的 的 了 是 没有复习价值。
    pub(super) fn record_encounter(&mut self, text: &str, translation: &Translation, sense: usize) {
        if self.private || !worth_encounter(text) {
            return;
        }
        let senses = translation.senses();
        let Some(sense) = senses.get(sense).or_else(|| senses.first()) else {
            return;
        };
        self.vocabulary.record_encounter(&Encounter {
            language: translation.language,
            text: text.to_owned(),
            translation: sense.text.clone(),
            reading: sense.reading.clone(),
        });
    }

    /// 上屏了一段没有现成译词的中文（整句、云端整句、分几次上屏拼成的一句）：释义兜底开着时交给它整段翻译，翻好了进相遇。
    pub(super) fn translate_for_encounter(&mut self, text: &str) {
        let language = self.translator.language();
        if self.private
            || language == Language::Chinese
            || !self.gloss_filler.is_enabled()
            || !worth_encounter(text)
        {
            return;
        }
        self.gloss_filler.translate(language, text);
    }

    /// 当前学习语言的词汇汇总（偏好设置「统计」页）。
    pub fn vocabulary_summary(&self) -> VocabularySummary {
        self.vocabulary.summary(self.translator.language())
    }

    /// 壳画完候选窗口后告知当前页上的候选：页上的译词在用户上屏那一刻记成「看到过」（[`super::vocabulary::VocabularyTracker`]）。
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
        if self.private {
            // 私密输入中的候选页不能污染词汇记录；同时丢掉暂存页，避免之后恢复普通输入时补记。
            self.displayed.clear();
            return;
        }
        for (language, word) in std::mem::take(&mut self.displayed) {
            self.vocabulary.record_exposure(language, &word);
        }
    }

    /// 上屏文字进本会话历史，也接到等着当例句的那句话后面。私密输入中的文字不进例句。
    pub(super) fn note_text(&mut self, text: &str) {
        self.history.record(text);
        if self.private {
            return;
        }
        let vocabulary = &mut self.vocabulary;
        let finished = self
            .examples
            .push_text(text, |language, word, source, example| {
                vocabulary.record_example(language, word, source, example);
            });
        for sentence in finished {
            self.translate_for_encounter(&sentence);
        }
    }

    /// 上文断开：手上那句话就算写完了。
    pub(super) fn finish_examples(&mut self) {
        let vocabulary = &mut self.vocabulary;
        let finished = self.examples.break_off(|language, word, source, example| {
            vocabulary.record_example(language, word, source, example);
        });
        if let Some(sentence) = finished {
            self.translate_for_encounter(&sentence);
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
    /// 从候选里选的再记是不是首选、这段敲了几键、打了多久、删过没有；部分上屏后下一段从这一刻接着数。
    pub(super) fn meter_commit(&mut self, text: &str, source: InputSource, english_word: bool) {
        if !self.metered {
            // 键数与时长也清掉，免得算到下一次记的上屏头上
            self.typing.take(Instant::now());
            return;
        }
        let mut usage = Usage::of_text(text);
        usage.words = match source {
            InputSource::Word | InputSource::Cloud => 1,
            InputSource::Sentence | InputSource::CloudSentence | InputSource::Voice => {
                sentence::segment_text(text, &*self.language_model).map_or(usage.hanzi, |clauses| {
                    clauses.iter().map(|words| words.len() as u64).sum()
                })
            }
            InputSource::English
            | InputSource::Custom
            | InputSource::Shortcut
            | InputSource::Emoji
            | InputSource::Raw
            | InputSource::Translation => 0,
        };
        if source == InputSource::Raw && !english_word {
            usage.english_words = 0;
        }
        let (keys, typing_ms, edited) = self.typing.take(Instant::now());
        // 原样上屏与语音不是从候选里选的：不算选词，也不计键数与时长
        if !matches!(source, InputSource::Raw | InputSource::Voice) {
            let first = self
                .last_query
                .borrow()
                .as_ref()
                .and_then(|snapshot| snapshot.candidates.first().map(|top| top == text))
                .unwrap_or(false);
            usage.choices = 1;
            usage.first_choices = u64::from(first);
            usage.edited = u64::from(edited);
            // 速度与码长只看出汉字的：英文候选敲了键却没有字
            if usage.hanzi > 0 {
                usage.typed_hanzi = usage.hanzi;
                usage.keys = keys;
                usage.typing_ms = typing_ms;
            }
        }
        self.meter.record(usage);
    }

    /// 往输入日志记一次上屏。`keys` 是这次消耗掉的原始键，`index` 从上一次查询的候选里找。
    /// 攒着的直通字符先写出去（顺序要对）；这段组句里退格过且最终键串变了，紧跟着记一条 `retype`。
    pub(super) fn log_commit(&mut self, keys: &str, text: &str, source: InputSource) -> u64 {
        self.record_exposures();
        self.flush_passthrough();
        self.log_sequence += 1;
        let snapshot = self.last_query.borrow().clone().unwrap_or_default();
        let index = snapshot.candidates.iter().position(|c| c == text);
        let ms = self
            .composition_started
            .map_or(0, |started| started.elapsed().as_millis() as u64);
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
            scheme: self.scheme_key(),
            english: self.english_mode,
            rescored: snapshot.rescored,
            choice: snapshot.choice,
            app: self.application.clone(),
            pages: self.page_turns,
            ms,
        }));
        self.committed_since_break = true;
        self.page_turns = 0;
        if let Some(before) = self.retype_snapshot.take() {
            let after = self.composition.scope();
            if before != after {
                self.logger.record(InputLogEntry::Retype {
                    before,
                    after: after.to_owned(),
                    of: self.log_sequence,
                });
            }
        }
        self.log_sequence
    }

    /// 用户要求删掉一个候选（修饰键 + 数字）：中文词与云端词交给 Learner 删用户词、清学习；英文词删个人英文词；
    /// 整句、快捷候选、emoji 没什么可删。删完缓存作废，它也不再当下一个词的上文。
    pub fn forget(&mut self, candidate: &Candidate) -> Forgotten {
        // 繁体输出开着时删的是简体那条：用户词与学习记录里存的都是简体
        let restored = self
            .traditional
            .restore(&candidate.text)
            .map(|text| Candidate {
                text,
                ..candidate.clone()
            });
        let candidate = restored.as_ref().unwrap_or(candidate);
        let forgotten = match candidate.kind {
            CandidateKind::Chinese | CandidateKind::Code | CandidateKind::Cloud => {
                self.learner.forget(&candidate.text)
            }
            CandidateKind::English => Forgotten {
                user_word: self.learner.forget_english(&candidate.text),
                learning: false,
            },
            CandidateKind::Sentence
            | CandidateKind::Shortcut
            | CandidateKind::Custom(_)
            | CandidateKind::Emoji
            | CandidateKind::Kaomoji
            | CandidateKind::Symbol
            | CandidateKind::Generated => Forgotten::default(),
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

/// 至少两个汉字才记相遇。
fn worth_encounter(text: &str) -> bool {
    text.chars()
        .filter(|c| sentence::is_han(*c))
        .nth(1)
        .is_some()
}
