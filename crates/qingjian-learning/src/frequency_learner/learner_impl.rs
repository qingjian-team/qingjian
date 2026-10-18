//! `Learner` trait 的实现：Engine 上屏 / 撤销 / 删候选时调进来的记账。

use super::*;

/// 这次选择要不要在读音键上另记一份：换个打法（`wod` / 双拼对全拼读音）也认账。
/// 候选盖满整段时读音与字面相同，只记一份；单字母简拼不泛化——那里翻页选词的意图太弱，
/// 带进全拼键会反过来压用户后来认真打的（`g` 选的 该 压住 `gai` 下的 改）。
fn generalizes(input: &str, reading: &str) -> bool {
    !reading.is_empty() && reading != input && input.len() >= 2
}

/// 选择的计数表里记一份；同一 (键, 词) 已有条目就累加。
fn bump_choice(choices: &mut HashMap<String, HashMap<String, u32>>, key: &str, text: &str) {
    *choices
        .entry(key.to_owned())
        .or_default()
        .entry(text.to_owned())
        .or_default() += 1;
}

/// 从选择的计数表里退回一份，空了连键一起删。
fn remove_choice(choices: &mut HashMap<String, HashMap<String, u32>>, key: &str, text: &str) {
    let Some(texts) = choices.get_mut(key) else {
        return;
    };
    if let Some(count) = texts.get_mut(text) {
        *count = count.saturating_sub(1);
        if *count == 0 {
            texts.remove(text);
        }
    }
    if texts.is_empty() {
        choices.remove(key);
    }
}

/// 键下的选择次数。
fn lookup_choice(choices: &HashMap<String, HashMap<String, u32>>, key: &str, text: &str) -> u32 {
    choices
        .get(key)
        .and_then(|texts| texts.get(text))
        .copied()
        .unwrap_or(0)
}

impl Learner for FrequencyLearner {
    fn record(&mut self, candidate: &Candidate) {
        self.tick();
        let now = self.today.get();
        self.counts
            .entry(candidate.text.clone())
            .or_insert_with(|| Decay::from_parts(0, now))
            .record(1, now);
        self.dirty = true;
        tracing::debug!(text = %candidate.text, "记录用户选择");
    }

    fn weight(&self, text: &str) -> u32 {
        self.counts
            .get(text)
            .map_or(0, |entry| self.effective(entry))
    }

    fn record_choice(&mut self, input: &str, reading: &str, text: &str) {
        if input.is_empty() || text.is_empty() {
            return;
        }
        self.tick();
        bump_choice(&mut self.choices, input, text);
        if generalizes(input, reading) {
            bump_choice(&mut self.choices, reading, text);
        }
        self.choices_dirty = true;
        if self.choice_count() > MAX_CHOICE_ENTRIES {
            self.decay_choices();
        }
        tracing::debug!(input, reading = %reading, text, "记录输入串下的选择");
    }

    fn choice_weight(&self, input: &str, text: &str) -> u32 {
        lookup_choice(&self.choices, input, text)
    }

    fn record_raw(&mut self, input: &str) {
        self.record_choice(input, "", RAW_MARK);
    }

    fn unrecord(&mut self, text: &str) {
        self.tick();
        if let Some(entry) = self.counts.get_mut(text) {
            entry.unrecord(1);
            if entry.count == 0 {
                self.counts.remove(text);
            }
            self.dirty = true;
            tracing::debug!(text, "撤销用户选择");
        }
    }

    fn unrecord_choice(&mut self, input: &str, reading: &str, text: &str) {
        remove_choice(&mut self.choices, input, text);
        if generalizes(input, reading) {
            remove_choice(&mut self.choices, reading, text);
        }
    }

    fn unrecord_transition(&mut self, context: Context<'_>, word: &str, times: u32) {
        self.ngram.unrecord(context, word, times);
        self.ngram_dirty = true;
    }

    fn raw_count(&self, input: &str) -> u32 {
        self.choice_weight(input, RAW_MARK)
    }

    fn learn_word(&mut self, text: &str, syllables: &[String]) {
        if text.is_empty() || syllables.is_empty() {
            return;
        }
        let pinyin = syllables.join(" ");
        if self.words.insert(text.to_owned(), pinyin) == Some(syllables.join(" ")) {
            return;
        }
        self.words_dirty = true;
        self.rebuild_words();
        tracing::debug!(text, ?syllables, "记录用户词");
    }

    fn user_words(&self) -> Option<&Dictionary> {
        self.user_dictionary.as_ref()
    }

    fn learn_english(&mut self, word: &str) {
        if word.is_empty() {
            return;
        }
        self.tick();
        let now = self.today.get();
        let entry = self
            .english
            .entry(word.to_ascii_lowercase())
            .or_insert_with(|| (word.to_owned(), Decay::from_parts(0, now)));
        entry.1.record(1, now);
        let count = entry.1.count;
        self.english_dirty = true;
        self.rebuild_english();
        tracing::debug!(word, count, "记录个人英文词");
    }

    fn user_english(&self) -> Option<&WordList> {
        self.english_list.as_ref()
    }

    fn record_transition(&mut self, context: Context<'_>, word: &str, times: u32) {
        if word.is_empty() || times == 0 {
            return;
        }
        self.ngram.record_times(context, word, times);
        self.ngram_dirty = true;
        tracing::debug!(?context, word, times, "记录词转移");
    }

    fn user_ngram(&self) -> Option<&UserNgram> {
        (!self.ngram.is_empty()).then_some(&self.ngram)
    }

    fn record_typo(&mut self, typed: &str, intended: &str) {
        if typed.is_empty() || intended.is_empty() {
            return;
        }
        self.tick();
        let now = self.today.get();
        self.typos
            .entry(typed.to_owned())
            .or_default()
            .entry(intended.to_owned())
            .or_insert_with(|| Decay::from_parts(0, now))
            .record(1, now);
        self.typos_dirty = true;
        tracing::debug!(typed, intended, "记录敲错");
    }

    fn unrecord_typo(&mut self, typed: &str, intended: &str) {
        let Some(intended_counts) = self.typos.get_mut(typed) else {
            return;
        };
        if let Some(entry) = intended_counts.get_mut(intended) {
            entry.unrecord(1);
            if entry.count == 0 {
                intended_counts.remove(intended);
            }
            self.typos_dirty = true;
        }
        if intended_counts.is_empty() {
            self.typos.remove(typed);
        }
    }

    fn typo_count(&self, typed: &str, intended: &str) -> u32 {
        self.typos
            .get(typed)
            .and_then(|intended_counts| intended_counts.get(intended))
            .map_or(0, |entry| self.effective(entry))
    }

    fn set_decay_half_life(&mut self, days: f64) {
        self.set_half_life_days(days);
    }

    fn forget(&mut self, text: &str) -> Forgotten {
        let mut forgotten = Forgotten::default();
        if self.words.remove(text).is_some() {
            forgotten.user_word = true;
            self.words_dirty = true;
            self.rebuild_words();
        }
        if self.counts.remove(text).is_some() {
            forgotten.learning = true;
            self.dirty = true;
        }
        let mut emptied = Vec::new();
        for (input, texts) in &mut self.choices {
            if texts.remove(text).is_some() {
                forgotten.learning = true;
                self.choices_dirty = true;
                if texts.is_empty() {
                    emptied.push(input.clone());
                }
            }
        }
        for input in emptied {
            self.choices.remove(&input);
        }
        if self.ngram.forget_word(text) > 0 {
            forgotten.learning = true;
            self.ngram_dirty = true;
        }
        tracing::debug!(text, ?forgotten, "删除词");
        forgotten
    }

    fn forget_english(&mut self, word: &str) -> bool {
        if self.english.remove(&word.to_ascii_lowercase()).is_none() {
            return false;
        }
        self.english_dirty = true;
        self.rebuild_english();
        true
    }

    fn flush(&mut self) {
        let Some(path) = self.path.clone() else {
            return;
        };
        self.tick();
        if self.dirty {
            match self.save_to(&path) {
                Ok(()) => {
                    tracing::info!(path = %path.display(), entries = self.counts.len(), "用户词频已保存")
                }
                Err(error) => tracing::warn!(path = %path.display(), %error, "用户词频保存失败"),
            }
        }
        if self.words_dirty {
            let words_path = Self::words_path(&path);
            match self.save_words_to(&words_path) {
                Ok(()) => {
                    tracing::info!(path = %words_path.display(), entries = self.words.len(), "用户词已保存")
                }
                Err(error) => {
                    tracing::warn!(path = %words_path.display(), %error, "用户词保存失败")
                }
            }
        }
        if self.english_dirty {
            let english_path = Self::english_path(&path);
            match self.save_english_to(&english_path) {
                Ok(()) => {
                    tracing::info!(path = %english_path.display(), entries = self.english.len(), "个人英文词已保存")
                }
                Err(error) => {
                    tracing::warn!(path = %english_path.display(), %error, "个人英文词保存失败")
                }
            }
        }
        if self.ngram_dirty {
            let ngram_path = Self::ngram_path(&path);
            match self.save_ngram_to(&ngram_path) {
                Ok(()) => {
                    tracing::info!(path = %ngram_path.display(), transitions = self.ngram.transition_count(), "个人 n-gram 已保存")
                }
                Err(error) => {
                    tracing::warn!(path = %ngram_path.display(), %error, "个人 n-gram 保存失败")
                }
            }
        }
        if self.choices_dirty {
            let choices_path = Self::choices_path(&path);
            match self.save_choices_to(&choices_path) {
                Ok(()) => {
                    tracing::info!(path = %choices_path.display(), entries = self.choice_count(), "输入串选择已保存")
                }
                Err(error) => {
                    tracing::warn!(path = %choices_path.display(), %error, "输入串选择保存失败")
                }
            }
        }
        if self.typos_dirty {
            let typos_path = Self::typos_path(&path);
            match self.save_typos_to(&typos_path) {
                Ok(()) => {
                    tracing::info!(path = %typos_path.display(), entries = self.typo_count_total(), "个人敲错表已保存")
                }
                Err(error) => {
                    tracing::warn!(path = %typos_path.display(), %error, "个人敲错表保存失败")
                }
            }
        }
    }
}
