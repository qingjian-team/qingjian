//! 上屏：译词标注、按候选消耗缓冲区、对齐音节、学习与撤销、自动造词。

use super::query::EnglishTail;
use super::*;

mod chain;
mod last;
mod transition;

pub(super) use chain::CommitChain;
pub use last::LastCommit;
pub use transition::Transition;

impl Engine {
    /// 给候选补上译文。与 [`Self::query`] 分开调用，平台层可以先画候选再补画译文。
    pub fn annotate(&self, list: &mut CandidateList) -> AnnotationReport {
        let start = Instant::now();
        let mut hits = 0;
        for candidate in &mut list.items {
            candidate.translation = match candidate.kind {
                // 英文候选按敲的大小写显示（Company / COMPANY），释义表键是小写
                CandidateKind::English => self
                    .english_translator
                    .translate(&candidate.text)
                    .or_else(|| {
                        self.english_translator
                            .translate(&candidate.text.to_ascii_lowercase())
                    }),
                _ => self
                    .translator
                    .translate(&candidate.text)
                    .map(|mut translation| {
                        self.mark_fresh(&mut translation);
                        translation
                    }),
            };
            hits += usize::from(candidate.translation.is_some());
        }
        AnnotationReport {
            total: list.items.len(),
            hits,
            elapsed: start.elapsed(),
        }
    }

    /// 上屏：记入学习，从缓冲区消耗掉该候选对应的拼音，返回要提交给应用的文本。
    ///
    /// 上屏候选的译文而不是候选本身（壳里修饰键 + 数字）：学习、拼音消耗都和选了这个候选一样，
    /// 返回第 `sense` 条释义的译文（0 是第一条，日文不带注音）。候选没有那么多条释义时不动，返回 `None`。
    pub fn commit_translation(&mut self, candidate: &Candidate, sense: usize) -> Option<String> {
        let text = candidate
            .translation
            .as_ref()
            .and_then(|t| t.senses().get(sense))
            .map(|s| s.text.clone())?;
        self.commit_with(candidate, InputSource::Translation, Some(sense));
        Some(text)
    }

    /// 候选比输入短时（`kaifazhe` 选了 开发），剩余拼音留在缓冲区，壳应接着 [`Self::query`]。
    /// 候选的最后一个音节比输入长时（`kaif` 选了 开发），把输入吃完。
    pub fn commit(&mut self, candidate: &Candidate) -> String {
        self.commit_with(candidate, InputSource::from(candidate.kind), None)
    }

    /// [`Self::commit`] 的内部形式：`source` 写进输入日志（上屏译词时不是候选本身），
    /// `used_sense` 是直接打出去的那条译词的序号（词汇记录里算「用过」）。
    pub(super) fn commit_with(
        &mut self,
        candidate: &Candidate,
        source: InputSource,
        used_sense: Option<usize>,
    ) -> String {
        // 整句不是一个词，不记词频；按路径上的词逐条记转移（喂个人 n-gram），路径要在拼音消耗前重算
        let sentence_words = (candidate.kind == CandidateKind::Sentence)
            .then(|| self.sentence_words(candidate))
            .flatten();
        // 下面每条路都可能改学习数据，格子候选的排序跟着变
        self.forget_span_cache();
        // 一段拼音里的第一个词：记下整段的学习键，整段分几次选完时合起来看（见 [`Self::finish_buffer`]）；
        // `split` 表示这次上屏接在同一段拼音里前一次上屏之后
        let split = self.chain.same_buffer();
        if !split {
            let (_, key) = self.whole_scope();
            self.chain.begin_buffer(key);
        }
        let mut typos = Vec::new();
        let (consumed, input) = match candidate.kind {
            CandidateKind::Chinese => {
                self.learner.record(candidate);
                let (consumed, input) = self.consumed_by(candidate);
                self.learner.record_choice(&input, &candidate.text);
                typos = self.accepted_typos(candidate);
                (consumed, input)
            }
            // 英文词带出的 emoji 没有音节，和英文词一样对应整段作用域
            CandidateKind::Emoji if candidate.syllables.is_empty() => self.whole_scope(),
            CandidateKind::Sentence => {
                typos = self.accepted_typos(candidate);
                self.consumed_by(candidate)
            }
            // emoji 按它对应词的音节消耗拼音，不记学习
            CandidateKind::Emoji => self.consumed_by(candidate),
            // 云端词是针对整段作用域要的（拼音可能有错，按音节对不上），上屏吃掉整段；词库里没有的记成用户词
            CandidateKind::Cloud => {
                if let Some(syllables) = self.learned_syllables(candidate) {
                    let learned = Candidate {
                        syllables,
                        ..candidate.clone()
                    };
                    if !self.knows_word(&learned) {
                        self.learner.learn_word(&learned.text, &learned.syllables);
                    }
                }
                self.learner.record(candidate);
                let (consumed, input) = self.whole_scope();
                self.learner.record_choice(&input, &candidate.text);
                (consumed, input)
            }
            // 英文词与快捷候选对应整段作用域；选中的英文词记次数并进个人英文词表，下次同样的前缀它靠前
            CandidateKind::English | CandidateKind::Shortcut => {
                if candidate.kind == CandidateKind::English {
                    self.learner.record(candidate);
                    self.learner.learn_english(&candidate.text);
                }
                self.whole_scope()
            }
        };
        self.apply_retraction(&input, &candidate.text);
        self.recording.clear();
        for (typed, intended) in &typos {
            tracing::debug!(typed, intended, "记录敲错");
            self.learner.record_typo(typed, intended);
        }
        let keys =
            self.composition.scope()[..consumed.min(self.composition.scope().len())].to_owned();
        let log_id = self.log_commit(&keys, &candidate.text, source);
        self.meter_commit(&candidate.text, source, false);
        // 上屏带译词的中文候选：那一刻用户看着这条译词，记进词汇（英文候选的中文释义不是学习语言，不记）
        if candidate.kind != CandidateKind::English
            && let Some(translation) = &candidate.translation
        {
            for (index, sense) in translation.senses().iter().enumerate() {
                self.vocabulary.record_commit(
                    translation.language,
                    &sense.text,
                    used_sense == Some(index),
                );
            }
        }
        // 词库里有、释义表里没有的词：交给释义兜底在后台问云端，写进个人释义表，下次就有译词；私密输入中不问
        if matches!(
            candidate.kind,
            CandidateKind::Chinese | CandidateKind::Cloud
        ) && self.gloss_filler.is_enabled()
            && !self.private
            && self.translator.language() != Language::Chinese
            && self.translator.translate(&candidate.text).is_none()
        {
            self.gloss_filler
                .request(self.translator.language(), &candidate.text);
        }
        self.composition.drain_prefix(consumed);
        let buffer_left = !self.composition.is_empty();
        match candidate.kind {
            CandidateKind::Chinese | CandidateKind::Cloud => {
                self.record_word(
                    &candidate.text,
                    &candidate.syllables,
                    EXPLICIT_TRANSITION_WEIGHT,
                    true,
                    buffer_left,
                );
            }
            CandidateKind::Sentence => match sentence_words {
                Some(words) => {
                    // 句末的英文词（我想学好rust 的 rust）记进个人英文词表，和英文候选上屏一样
                    if let Some(word) = words.last().filter(|w| is_english_word(w)) {
                        self.learner.learn_english(&word.text);
                    }
                    // 紧接着同一段拼音里自选的词（`jidiaole` 选了 挤，剩下的 掉了 走整句）：接缝是用户自己定的，
                    // 第一个词的转移按自选记双份。不参与两词造词：我 + 的… 这种接缝太常见、转移计数早就够了，
                    // 会把 我的 一类造成用户词；整段合成词由 [`Self::finish_buffer`] 管
                    let junction = self.chain.same_buffer() && self.chain.previous().is_some();
                    let last = words.len() - 1;
                    for (index, word) in words.iter().enumerate() {
                        let times = if index == 0 && junction {
                            EXPLICIT_TRANSITION_WEIGHT
                        } else {
                            1
                        };
                        self.record_word(
                            &word.text,
                            &word.syllables,
                            times,
                            false,
                            buffer_left || index < last,
                        );
                    }
                }
                None => self.chain.reset(),
            },
            CandidateKind::English | CandidateKind::Shortcut | CandidateKind::Emoji => {
                self.chain.reset()
            }
        }
        // 一次整句上屏里的几个词不算分段选，只有这段拼音经过至少两次上屏才合起来看
        let phrase = if split && !buffer_left {
            self.finish_buffer()
        } else {
            None
        };
        self.punctuation.note_committed(&candidate.text);
        self.history.record(&candidate.text);
        let learned = matches!(
            candidate.kind,
            CandidateKind::Chinese | CandidateKind::Cloud | CandidateKind::Sentence
        );
        let commit = if learned {
            LastCommit {
                text: candidate.text.clone(),
                chars: candidate.text.chars().count(),
                input,
                chosen: matches!(
                    candidate.kind,
                    CandidateKind::Chinese | CandidateKind::Cloud
                )
                .then(|| candidate.text.clone()),
                transitions: std::mem::take(&mut self.recording),
                typos,
                erased: 0,
                log_id,
                phrase,
            }
        } else {
            LastCommit::plain(&candidate.text)
        };
        self.remember_commit(commit);
        candidate.text.clone()
    }

    /// 一段拼音分几次选完了（`jidiaole` 先选 挤、剩下的走整句 掉了）：这几个词合起来就是用户对这段拼音的答案。
    /// 记一次「整段拼音 → 合成词」的选择；选到 [`AUTO_WORD_THRESHOLD_SAME_BUFFER`] 次、词库里没有、
    /// 不超过 [`AUTO_WORD_MAX_CHARS`] 字就造成用户词，下次整段打出来它直接排第一。返回记下的选择，撤销时退回。
    pub(super) fn finish_buffer(&mut self) -> Option<(String, String)> {
        let words = self.chain.buffer_words();
        if words.len() < 2 {
            return None;
        }
        let text: String = words.iter().map(|(t, _)| t.as_str()).collect();
        let syllables: Vec<String> = words.iter().flat_map(|(_, s)| s.iter().cloned()).collect();
        let chars = text.chars().count();
        let key = self.chain.buffer_key().to_owned();
        if key.is_empty() || chars > AUTO_WORD_MAX_CHARS || chars != syllables.len() {
            return None;
        }
        self.learner.record_choice(&key, &text);
        let candidate = Candidate {
            text,
            kind: CandidateKind::Chinese,
            syllables,
            reading: None,
            translation: None,
        };
        if !self.knows_word(&candidate)
            && self.learner.choice_weight(&key, &candidate.text) >= AUTO_WORD_THRESHOLD_SAME_BUFFER
        {
            tracing::debug!(text = %candidate.text, "整段拼音分次选完，自动造词");
            self.learner
                .learn_word(&candidate.text, &candidate.syllables);
        }
        Some((key, candidate.text))
    }

    /// 最近删掉的上屏里有一次是同一段拼音（或它的前缀）、这次却选了别的词：把那次记的学习退回去。
    /// 删掉几个词再从头重打是常事（「沃德 书」删掉重打成「我的 书」），所以往前找最近一次删干净的同段拼音；
    /// 重打后选的还是同一个词就不算选错，只把那条记录丢掉。这次的拼音与删掉的哪次都对不上时，删掉的那些当作在改别处，全忘掉。
    pub(super) fn apply_retraction(&mut self, input: &str, text: &str) {
        let Some(index) = self
            .recent_commits
            .iter()
            .rposition(|c| c.is_erased() && c.same_input(input))
        else {
            // 删掉的那次拼音与这次相近但不同（wode 删了重打 wodi）：重打了键，记进输入日志给敲错纠正当样本；学习不动
            if let Some(erased) = self
                .recent_commits
                .iter()
                .rev()
                .find(|c| c.is_erased() && is_retyped(&c.input, input))
            {
                self.logger.record(InputLogEntry::Retype {
                    before: erased.input.clone(),
                    after: input.to_owned(),
                    of: erased.log_id,
                });
            }
            self.recent_commits.retain(|c| !c.is_erased());
            return;
        };
        let last = self.recent_commits.remove(index);
        if last.text == text {
            return;
        }
        tracing::debug!(retracted = %last.text, chosen = %text, "上次选错了，撤销它的学习");
        self.logger.record(InputLogEntry::Retract {
            of: last.log_id,
            text: last.text.clone(),
            chosen: text.to_owned(),
        });
        if let Some(chosen) = &last.chosen {
            self.learner.unrecord(chosen);
            self.learner.unrecord_choice(&last.input, chosen);
        }
        for transition in &last.transitions {
            self.learner.unrecord_transition(
                transition.context(),
                &transition.word,
                transition.times,
            );
        }
        for (typed, intended) in &last.typos {
            self.learner.unrecord_typo(typed, intended);
        }
        if let Some((key, phrase)) = &last.phrase {
            self.learner.unrecord_choice(key, phrase);
        }
    }

    /// 这个候选上屏算接受了哪些音节级敲错：词图里靠敲错变体对上的音节，加上整段纠错落在的那个音节（吃到了编辑处才算）。
    /// 双拼不记（键与全拼对不上）。
    pub(super) fn accepted_typos(&self, candidate: &Candidate) -> Vec<(String, String)> {
        let keys = self.composition.scope();
        if self.decode(keys).is_some() {
            return Vec::new();
        }
        match self.active_correction(keys) {
            Some(c) => {
                let alignment = self.align(&c.corrected, &candidate.syllables);
                let mut typos = alignment.typos;
                typos.extend(c.typo_pair(alignment.consumed));
                typos
            }
            None => self.align(keys, &candidate.syllables).typos,
        }
    }

    /// 整句候选对应的词序列：重算一次整句转换，文本对得上才算（对不上说明候选来自别处，不记）。
    /// 末尾是英文词的整句（我想学好rust）先按头段试，英文词作为最后一个词（音节就是敲的字母）。
    pub(super) fn sentence_words(
        &self,
        candidate: &Candidate,
    ) -> Option<Vec<sentence::SentenceWord>> {
        let scope = self.composition.scope();
        if self.decode(scope).is_none()
            && let Some(tail) = self.split_english_tail(scope)
            && let Some(words) = self.mixed_words(scope, &tail, &candidate.text)
        {
            return Some(words);
        }
        let conversion = match (self.decode(scope), self.active_correction(scope)) {
            (Some(decoded), _) => {
                self.convert_sentence(&decoded.segmentation()?.patterns(), true)?
            }
            (None, Some(c)) => self.convert_sentence(&c.segmentation.patterns(), false)?,
            (None, None) => {
                let (segmentations, _) = segment_longest_prefix(scope).ok()?;
                self.convert_sentence(&segmentations.first()?.patterns(), true)?
            }
        };
        (conversion.text == candidate.text).then_some(conversion.words)
    }

    /// 头段拼音的转换加上英文尾段，与 `text` 对得上时的词序列。
    fn mixed_words(
        &self,
        scope: &str,
        tail: &EnglishTail,
        text: &str,
    ) -> Option<Vec<sentence::SentenceWord>> {
        let segmentations = parser::segment(&scope[..tail.head_len]).ok()?;
        let mut conversion = self.convert_sentence(&segmentations.first()?.patterns(), true)?;
        conversion.text.push_str(&tail.word);
        if conversion.text != text {
            return None;
        }
        conversion.words.push(sentence::SentenceWord {
            text: tail.word.clone(),
            syllables: vec![scope[tail.head_len..].to_owned()],
            placeholder: false,
        });
        Some(conversion.words)
    }

    /// 候选消耗多少作用域字节，以及按输入串记学习用的键（候选覆盖的那段全拼字母）。
    /// 纠错生效时按纠正后的拼音算，再按那处编辑换算回原串；双拼按解出的全拼算，再换算回键数。
    pub(super) fn consumed_by(&self, candidate: &Candidate) -> (usize, String) {
        let keys = self.composition.scope();
        if let Some(decoded) = self.decode(keys) {
            let pinyin_len = self.align(decoded.pinyin(), &candidate.syllables).consumed;
            return (
                decoded.keys_for(pinyin_len),
                choice_key(decoded.pinyin(), pinyin_len),
            );
        }
        let consumed = match self.active_correction(keys) {
            Some(c) => c
                .edit
                .to_original(self.align(&c.corrected, &candidate.syllables).consumed)
                .min(keys.len()),
            None => self.align(keys, &candidate.syllables).consumed,
        };
        (consumed, choice_key(keys, consumed))
    }

    /// 候选的音节逐个对到 `input` 上：原样相同直接吃；输入到这里就没了而且是这个音节的开头算没打完；
    /// 否则找最长的一段字母是它的模糊音或敲错变体（`zi` 对 `zhi`、`gan` 对 `guan`）；都不是就按公共前缀吃。`'` 分隔的一段字母不跨段对。
    pub(super) fn align(&self, input: &str, syllables: &[String]) -> Alignment {
        let mut alignment = Alignment::default();
        let mut pos = 0;
        for syllable in syllables {
            if pos > 0 && input[pos..].starts_with('\'') {
                pos += 1;
            }
            let rest = &input[pos..];
            let rest = &rest[..rest.find('\'').unwrap_or(rest.len())];
            if rest.starts_with(syllable.as_str()) {
                pos += syllable.len();
                continue;
            }
            if !rest.is_empty() && syllable.starts_with(rest) {
                pos += rest.len();
                continue;
            }
            let longest = (1..=rest.len().min(parser::MAX_SYLLABLE_LEN))
                .rev()
                .find(|&len| {
                    let typed = &rest[..len];
                    self.fuzzy.is_variant(typed, syllable) || typo::is_variant(typed, syllable)
                });
            if let Some(len) = longest {
                let typed = &rest[..len];
                if !self.fuzzy.is_variant(typed, syllable) {
                    alignment.typos.push((typed.to_owned(), syllable.clone()));
                }
                pos += len;
                continue;
            }
            let common = syllable
                .bytes()
                .zip(rest.bytes())
                .take_while(|(a, b)| a == b)
                .count();
            if common == 0 {
                break;
            }
            pos += common;
        }
        alignment.consumed = pos;
        alignment
    }

    /// 整段作用域对应的候选（英文词、云端词、快捷候选）：吃掉全部键，学习键是整段全拼。
    pub(super) fn whole_scope(&self) -> (usize, String) {
        let keys = self.composition.scope();
        let input = match self.decode(keys) {
            Some(decoded) => choice_key(decoded.pinyin(), decoded.pinyin().len()),
            None => choice_key(keys, keys.len()),
        };
        (keys.len(), input)
    }

    /// 一个中文词上屏了：记 `times` 份转移、推进链；`auto_word` 为真（用户自己选的词）时，
    /// 紧接着上一个词、合起来词库里没有、且这条接续记够次数还自动造词。
    pub(super) fn record_word(
        &mut self,
        text: &str,
        syllables: &[String],
        times: u32,
        auto_word: bool,
        buffer_left: bool,
    ) {
        self.learner
            .record_transition(self.chain.context(), text, times);
        self.recording
            .push(Transition::new(self.chain.context(), text, times));
        if auto_word {
            let threshold = if self.chain.same_buffer() {
                AUTO_WORD_THRESHOLD_SAME_BUFFER
            } else {
                AUTO_WORD_THRESHOLD
            };
            self.try_auto_word(text, syllables, threshold);
        }
        self.chain.advance(text, syllables, buffer_left);
    }

    /// 上一个词 + 这个词合成用户词的条件见 [`AUTO_WORD_THRESHOLD`]。
    pub(super) fn try_auto_word(&mut self, text: &str, syllables: &[String], threshold: u32) {
        let Some(previous) = self.chain.previous().map(str::to_owned) else {
            return;
        };
        let joined = format!("{previous}{text}");
        let chars = joined.chars().count();
        let mut joined_syllables = self.chain.previous_syllables().to_vec();
        joined_syllables.extend(syllables.iter().cloned());
        if chars > AUTO_WORD_MAX_CHARS || chars != joined_syllables.len() {
            return;
        }
        // 这条转移刚记过，计数已含本次；阈值按「选了几次」算，计数是按份记的
        let seen = self
            .learner
            .user_ngram()
            .map_or(0, |b| b.pair(Some(&previous), text));
        if seen < threshold * EXPLICIT_TRANSITION_WEIGHT {
            return;
        }
        let candidate = Candidate {
            text: joined,
            kind: CandidateKind::Chinese,
            syllables: joined_syllables,
            reading: None,
            translation: None,
        };
        if self.knows_word(&candidate) {
            return;
        }
        tracing::debug!(text = %candidate.text, "自动造词");
        self.learner
            .learn_word(&candidate.text, &candidate.syllables);
    }

    /// 主词库或用户词里是否已有这个词（同音节）。
    pub(super) fn knows_word(&self, candidate: &Candidate) -> bool {
        let syllables: Vec<&str> = candidate.syllables.iter().map(String::as_str).collect();
        if syllables.is_empty() {
            return true;
        }
        let known = |dictionary: &Dictionary| {
            dictionary
                .lookup(&syllables, false)
                .iter()
                .any(|hit| hit.exact && hit.text == candidate.text)
        };
        self.all_dictionaries().into_iter().any(known)
    }
}

/// 整句路径上的词是英文词（`woxiangxuehaorust` 的 rust）：不是占位音节、全是字母。
fn is_english_word(word: &sentence::SentenceWord) -> bool {
    !word.placeholder && !word.text.is_empty() && word.text.bytes().all(|b| b.is_ascii_alphabetic())
}

/// 删掉重打的键串算不算「同一段拼音打错了」：都够长、不相等、编辑距离不超过 [`RETYPE_MAX_EDITS`]。
/// 差得更多的是在改别的内容，不算。
fn is_retyped(before: &str, after: &str) -> bool {
    before.len() >= RETYPE_MIN_LETTERS
        && after.len() >= RETYPE_MIN_LETTERS
        && before != after
        && edit_distance(before, after) <= RETYPE_MAX_EDITS
}

/// 跨上屏重打的键串最少几个字母才算数。
const RETYPE_MIN_LETTERS: usize = 3;

/// 跨上屏重打最多差几处编辑。
const RETYPE_MAX_EDITS: usize = 2;

/// Levenshtein 编辑距离（按字节，键串都是 ASCII）。
fn edit_distance(a: &str, b: &str) -> usize {
    let a = a.as_bytes();
    let b = b.as_bytes();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current = vec![0; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        current[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            current[j + 1] = (previous[j] + cost)
                .min(previous[j + 1] + 1)
                .min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b.len()]
}
