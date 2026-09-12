//! 联想：候选之外的异步补充。
//!
//! [`Predictor`] 由壳注入（网络实现在 `qingjian-predict`，Core 永远不联网），Engine 负责裁剪上下文、
//! 编号请求、校验云端词的拼音、丢弃过期结果。联想**不参与排序、不阻塞输入**；
//! 云端词到了只补进候选窗口第一页末尾几格（[`crate::CandidateLayout`]），前面的本地候选不挪。
//!
//! Engine 这一侧的实现：组句中发请求、收结果校验、Tab 接受整句；翻译选中文字也走这里。

mod cloud_word;
mod fuzzy;
mod kind;
mod policy;
mod predictor;
mod question;
mod request;
mod response;
mod script;
mod surrounding_text;

pub use cloud_word::CloudWord;
pub use fuzzy::{mismatch_count, tolerance};
pub use kind::PredictionKind;
pub use policy::PredictionPolicy;
pub use predictor::{NoPredictor, Predictor};
pub use question::restates_question;
pub use request::PredictionRequest;
pub use response::Prediction;
pub use script::translation_target;
pub use surrounding_text::SurroundingText;

use super::*;

impl Engine {
    /// 是否接了会联想的 Predictor；壳据此决定要不要起轮询定时器、画云朵标识。
    pub fn prediction_enabled(&self) -> bool {
        self.predictor.is_enabled()
    }

    pub fn prediction_policy(&self) -> PredictionPolicy {
        self.predictor.policy()
    }

    /// 发一次联想请求，返回序号；没接 Predictor、私密输入中或拼音太短时不发，返回 `None`。
    ///
    /// 要的是「当前作用域拼音对应的词」和整句补全；`candidates` 是本地候选，只取前几个当提示。
    /// `surrounding` 是应用给的光标前后文本，在这里按观察窗口裁剪，壳给得再多也只发这么多；
    /// 应用给不出上下文就只靠拼音，本地输入历史不可靠、不用。上屏之后不联想：没有拼音约束的下文联想只是噪音。
    pub fn request_prediction(
        &mut self,
        surrounding: Option<SurroundingText>,
        candidates: &[Candidate],
    ) -> Option<u64> {
        if !self.predictor.is_enabled() || self.private {
            return None;
        }
        // 不发也要换序号：正在飞的旧结果对应的是上一个输入状态，回来了也不能显示
        self.prediction_sequence += 1;
        let policy = self.predictor.policy();
        let (before, after) = match surrounding {
            Some(text) => (
                take_last_chars(&text.before, policy.before),
                take_first_chars(&text.after, policy.after),
            ),
            None => (String::new(), String::new()),
        };
        let scope = self.composition.scope();
        if self.english_mode
            || self.modes().is_expression(scope)
            || is_raw(scope, self.modes(), self.shuangpin)
        {
            return None;
        }
        // 问字模式：问题本身就是全部上下文，不带应用文本、不要整句、本地没有候选可提示
        let question = self.modes().is_question(scope);
        if question && shortcut::unicode_form(self.modes().question_body(scope)).is_some() {
            // 码点输入本地就能答，不问云端
            return None;
        }
        let (kind, pinyin_source, before, after) = if question {
            (
                PredictionKind::Question,
                self.modes().question_body(scope),
                String::new(),
                String::new(),
            )
        } else {
            (PredictionKind::Compose, scope, before, after)
        };
        // 双拼：问云端用的是解出来的全拼，不是敲的键
        let decoded = self.decode(pinyin_source);
        let pinyin_source: &str = decoded.as_ref().map_or(pinyin_source, |d| d.pinyin());
        let letters = pinyin_source.chars().filter(|c| *c != '\'').count();
        if letters < MIN_PREDICTION_LETTERS {
            return None;
        }
        let (pinyin, syllables, guess) = match segment_longest_prefix(pinyin_source) {
            Ok((segmentations, tail)) => (
                query::join_marked(&segmentations, tail),
                segmentations.first().map_or(0, |s| s.syllables.len()),
                self.local_guess(&segmentations),
            ),
            Err(_) => (pinyin_source.to_owned(), 0, String::new()),
        };
        if question {
            self.last_question_guess = guess.clone();
        }
        let request = PredictionRequest {
            sequence: self.prediction_sequence,
            kind,
            before,
            after,
            pinyin,
            letters: pinyin_source.replace('\'', ""),
            syllables,
            candidates: candidates
                .iter()
                .take(PREDICTION_CANDIDATE_HINTS)
                .map(|c| c.text.clone())
                .collect(),
            guess,
            max_items: policy.max_items,
            want_sentence: policy.sentence && !question,
            text: String::new(),
            target_language: String::new(),
        };
        self.last_prediction_kind = kind;
        self.last_prediction_scope = scope.to_owned();
        self.predictor.submit(request);
        Some(self.prediction_sequence)
    }

    /// 把应用里选中的一段文字交给云端翻译（壳里快捷键触发）：主要是汉字就译成学习语言，是外文（拉丁字母、假名）就译成中文
    /// （[`translation_target`]）。云联想关着、私密输入中、文字为空时不发，返回 `None`；
    /// 译文从 [`Self::poll_prediction`] 的 `sentence` 里出。不进学习、不动缓冲区。
    pub fn request_translation(&mut self, text: &str) -> Option<u64> {
        let text = text.trim();
        if !self.predictor.is_enabled() || self.private || text.is_empty() {
            return None;
        }
        self.prediction_sequence += 1;
        let request = PredictionRequest {
            sequence: self.prediction_sequence,
            kind: PredictionKind::Translate,
            before: String::new(),
            after: String::new(),
            pinyin: String::new(),
            letters: String::new(),
            syllables: 0,
            candidates: Vec::new(),
            guess: String::new(),
            max_items: 1,
            want_sentence: true,
            text: text.to_owned(),
            target_language: translation_target(text, self.translator.language())
                .code()
                .to_owned(),
        };
        self.last_prediction_kind = PredictionKind::Translate;
        self.predictor.submit(request);
        Some(self.prediction_sequence)
    }

    /// 作废正在飞的联想（用户清空了拼音、关掉了联想框）。
    pub fn cancel_prediction(&mut self) {
        self.prediction_sequence += 1;
    }

    /// 取回最近一次请求的结果；过期结果直接丢。没有就绪的结果返回 `None`，不阻塞。
    /// 结果可能是空的（模型没给出可用条目），壳据此停止等待。
    pub fn poll_prediction(&mut self) -> Option<Prediction> {
        while let Some(mut prediction) = self.predictor.poll() {
            if prediction.sequence == self.prediction_sequence {
                // 问字的答案、翻译的译文和敲的拼音本来就对不上，只有组句联想的云端词要校验
                if self.last_prediction_kind == PredictionKind::Question {
                    let guess = &self.last_question_guess;
                    prediction
                        .words
                        .retain(|word| !restates_question(&word.text, guess));
                } else if self.last_prediction_kind == PredictionKind::Compose
                    && !self.question_mode()
                {
                    if self.predictor.policy().slots == 0 {
                        // 用户不要云端词，只留整句补全
                        prediction.words.clear();
                    } else {
                        self.validate_cloud_words(&mut prediction.words);
                    }
                    if !prediction.is_empty() {
                        // 给过用户什么：紧接着的上屏说明接没接受（本地联想能不能替代云端的尺子）
                        self.logger.record(InputLogEntry::Prediction {
                            scope: self.last_prediction_scope.clone(),
                            words: prediction.words.iter().map(|w| w.text.clone()).collect(),
                            sentence: prediction.sentence.clone(),
                        });
                    }
                }
                return Some(prediction);
            }
            tracing::debug!(
                sequence = prediction.sequence,
                current = self.prediction_sequence,
                "丢弃过期的联想结果"
            );
        }
        None
    }

    /// 只留下拼音对得上的云端词：字数等于音节数、每个音节合法、全拼与用户敲的字母的编辑距离在容许范围内
    /// （简拼不算错，允许少量错字 / 漏字 / 多字，纠错就靠这个）。模型偶尔会给出根本不是这个拼音的词，这些不进候选。
    pub(super) fn validate_cloud_words(&self, words: &mut Vec<CloudWord>) {
        let decoded = self.decode(self.composition.scope());
        let typed = decoded
            .as_ref()
            .map_or(self.composition.scope(), |d| d.pinyin());
        let letters = typed.chars().filter(|c| *c != '\'').count();
        let allowed = tolerance(letters);
        words.retain(|word| {
            let fits = !word.syllables.is_empty()
                && word.text.chars().count() == word.syllables.len()
                && word.syllables.iter().all(|s| parser::is_syllable(s))
                && mismatch_count(typed, &word.syllables) <= allowed;
            if !fits {
                tracing::debug!(text = %word.text, syllables = ?word.syllables, "云端词与拼音不符，丢弃");
            }
            fits
        });
    }

    /// 用户接受了一条整句补全：作用域内的拼音作废、句子上屏。句子没有拼音，记不了词频与用户词，
    /// 但按语言模型把它切成词（[`sentence::segment_text`]）逐条记进个人 n-gram，与选整句候选一样；
    /// 标点处断句，句尾是标点时之后的词按句首记。整句退格删光再重打时这些转移一并退回。
    pub fn accept_prediction(&mut self, text: &str) -> String {
        let (_, input) = self.whole_scope();
        self.apply_retraction(&input, text);
        self.recording.clear();
        let keys = self.composition.scope().to_owned();
        let log_id = self.log_commit(&keys, text, InputSource::CloudSentence);
        self.meter_commit(text, InputSource::CloudSentence, false);
        self.composition.drain_scope();
        self.punctuation.note_committed(text);
        self.history.record(text);
        match sentence::segment_text(text, &*self.language_model) {
            Some(clauses) => {
                let buffer_left = !self.composition.is_empty();
                let count = clauses.len();
                for (index, words) in clauses.iter().enumerate() {
                    // 第一句接着前面上屏的词；标点之后的各句从句首起
                    if index > 0 {
                        self.chain.reset();
                    }
                    for word in words {
                        self.record_word(word, &[], 1, false, buffer_left || index + 1 < count);
                    }
                }
                if text.chars().last().is_some_and(|c| !c.is_alphanumeric()) {
                    self.chain.reset();
                }
                tracing::debug!(clauses = count, "整句补全已记入个人 n-gram");
            }
            None => self.chain.reset(),
        }
        let commit = LastCommit {
            text: text.to_owned(),
            chars: text.chars().count(),
            input,
            chosen: None,
            transitions: std::mem::take(&mut self.recording),
            typos: Vec::new(),
            erased: 0,
            log_id,
            phrase: None,
        };
        self.remember_commit(commit);
        text.to_owned()
    }

    /// 云端词学成用户词时用哪套音节。模型给的读音偶有错（我的 → wo di），错读音学进去以后只会按错读音出来，
    /// 还会把整句候选里正确的同文本词顶掉；用户敲的拼音能切成与字数相同的完整音节、每个又是对应字的读音时以敲的为准，
    /// 否则仍用模型的，但模型的读音里有词库不认「这个字读这个音」的就不学。
    pub(super) fn learned_syllables(&self, candidate: &Candidate) -> Option<Vec<String>> {
        let chars: Vec<String> = candidate.text.chars().map(String::from).collect();
        let decoded = self.decode(self.composition.scope());
        let typed = decoded
            .as_ref()
            .map_or(self.composition.scope(), |d| d.pinyin());
        if let Ok((segmentations, "")) = segment_longest_prefix(typed)
            && let Some(best) = segmentations.first()
            && best.syllables.len() == chars.len()
            && best
                .syllables
                .iter()
                .zip(&chars)
                .all(|(s, c)| s.complete && self.accepts_reading(c, &s.text))
        {
            return Some(best.syllables.iter().map(|s| s.text.clone()).collect());
        }
        let fits = candidate.syllables.len() == chars.len()
            && candidate
                .syllables
                .iter()
                .zip(&chars)
                .all(|(s, c)| self.accepts_reading(c, s));
        fits.then(|| candidate.syllables.clone())
    }

    /// 这个字按这个读音能不能接受：词库里这个字有这个读音；词库根本不认识这个字（生僻字）时没法核对，照单全收。
    fn accepts_reading(&self, ch: &str, syllable: &str) -> bool {
        self.char_reads(ch, syllable) || !parser::SYLLABLES.iter().any(|s| self.char_reads(ch, s))
    }

    /// 词库（含用户词）里这个字有没有这个读音。
    fn char_reads(&self, ch: &str, syllable: &str) -> bool {
        let pattern = [qingjian_dictionary::SyllablePattern {
            text: syllable,
            complete: true,
        }];
        self.all_dictionaries()
            .into_iter()
            .any(|d| d.lookup_exact(&pattern).iter().any(|hit| hit.text == ch))
    }
}
