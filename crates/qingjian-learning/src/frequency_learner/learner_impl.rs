//! `Learner` trait 的实现：Engine 上屏 / 撤销 / 删候选时调进来的记账。

use super::*;

impl Learner for FrequencyLearner {
    fn record(&mut self, candidate: &Candidate) {
        *self.counts.entry(candidate.text.clone()).or_default() += 1;
        self.dirty = true;
        tracing::debug!(text = %candidate.text, "记录用户选择");
    }

    fn weight(&self, text: &str) -> u32 {
        self.counts.get(text).copied().unwrap_or(0)
    }

    fn record_choice(&mut self, input: &str, text: &str) {
        if input.is_empty() || text.is_empty() {
            return;
        }
        *self
            .choices
            .entry(input.to_owned())
            .or_default()
            .entry(text.to_owned())
            .or_default() += 1;
        self.choices_dirty = true;
        if self.choice_count() > MAX_CHOICE_ENTRIES {
            self.decay_choices();
        }
        tracing::debug!(input, text, "记录输入串下的选择");
    }

    fn choice_weight(&self, input: &str, text: &str) -> u32 {
        self.choices
            .get(input)
            .and_then(|texts| texts.get(text))
            .copied()
            .unwrap_or(0)
    }

    fn record_raw(&mut self, input: &str) {
        self.record_choice(input, RAW_MARK);
    }

    fn unrecord(&mut self, text: &str) {
        if let Some(count) = self.counts.get_mut(text) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.counts.remove(text);
            }
            self.dirty = true;
            tracing::debug!(text, "撤销用户选择");
        }
    }

    fn unrecord_choice(&mut self, input: &str, text: &str) {
        let Some(texts) = self.choices.get_mut(input) else {
            return;
        };
        if let Some(count) = texts.get_mut(text) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                texts.remove(text);
            }
            self.choices_dirty = true;
        }
        if texts.is_empty() {
            self.choices.remove(input);
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
        let entry = self
            .english
            .entry(word.to_ascii_lowercase())
            .or_insert_with(|| (word.to_owned(), 0));
        entry.1 += 1;
        let count = entry.1;
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
        *self
            .typos
            .entry(typed.to_owned())
            .or_default()
            .entry(intended.to_owned())
            .or_default() += 1;
        self.typos_dirty = true;
        tracing::debug!(typed, intended, "记录敲错");
    }

    fn unrecord_typo(&mut self, typed: &str, intended: &str) {
        let Some(intended_counts) = self.typos.get_mut(typed) else {
            return;
        };
        if let Some(count) = intended_counts.get_mut(intended) {
            *count = count.saturating_sub(1);
            if *count == 0 {
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
            .copied()
            .unwrap_or(0)
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
