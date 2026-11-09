//! 上屏后的联想：一个中文词上屏、组句也结束了，列出最可能接在它后面的词，点了直接上屏（Android 候选条用）。
//!
//! 先列用户自己在这个词后面接过的（个人 n-gram），再补语言模型里最常见的接续；只给全是汉字、不超过
//! [`MAX_FOLLOW_UP_CHARS`] 个字的词。上屏的联想词照常记词频与转移，并接着推进上文，可以一路点下去。

use super::commit::LastCommit;
use super::input_log::InputSource;
use super::learning::Learner;
use super::{EXPLICIT_TRANSITION_WEIGHT, Engine};
use crate::candidate::{Candidate, CandidateKind};

/// 联想词最长几个字：再长的多半是语言模型里的短语片段，读起来不像一个词。
const MAX_FOLLOW_UP_CHARS: usize = 4;

impl Engine {
    /// 接在刚上屏的中文词后面的联想，最多 `limit` 个；上文断了（句首、标点、英文、回车上屏拼音之后）或还在组句时为空。
    /// 繁体输出开着时给繁体。
    pub fn follow_ups(&self, limit: usize) -> Vec<String> {
        if limit == 0 || !self.composition.is_empty() {
            return Vec::new();
        }
        let Some(previous) = self.chain.previous() else {
            return Vec::new();
        };
        let fits = |word: &str| {
            let chars = word.chars().count();
            (1..=MAX_FOLLOW_UP_CHARS).contains(&chars) && word.chars().all(crate::sentence::is_han)
        };
        let mut words: Vec<String> = Vec::new();
        if let Some(ngram) = self.learner.user_ngram() {
            for (word, _) in ngram.successors(previous) {
                if words.len() >= limit {
                    break;
                }
                if fits(word) {
                    words.push(word.to_owned());
                }
            }
        }
        // 多取一些：过滤掉的（标点、英文、长片段）不占名额
        for word in self.language_model.successors(previous, limit * 4) {
            if words.len() >= limit {
                break;
            }
            if fits(&word) && !words.contains(&word) {
                words.push(word);
            }
        }
        words
            .into_iter()
            .map(|word| self.traditional.convert(&word))
            .collect()
    }

    /// 点了联想词 `text`（[`Self::follow_ups`] 给的那份，繁体输出时是繁体）：记词频与转移、推进上文，返回交给应用的文字。
    /// 不算选词、不计键数：没有敲键。
    pub fn commit_follow_up(&mut self, text: &str) -> String {
        if text.is_empty() {
            return String::new();
        }
        let word = self
            .traditional
            .restore(text)
            .unwrap_or_else(|| text.to_owned());
        *self.last_query.borrow_mut() = None;
        self.learner.record(&Candidate {
            text: word.clone(),
            kind: CandidateKind::Chinese,
            syllables: Vec::new(),
            reading: None,
            translation: None,
            aux_code: None,
        });
        // 没有音节：联想词不参与两词自动造词（`try_auto_word` 要字数与音节数对上）
        self.record_word(&word, &[], EXPLICIT_TRANSITION_WEIGHT, false, false);
        self.recording.clear();
        self.log_commit("", &word, InputSource::FollowUp);
        self.meter_commit(&word, InputSource::FollowUp, false);
        self.remember_commit(LastCommit::plain(&word));
        self.punctuation.note_committed(&word);
        self.note_text(&word);
        self.traditional.convert(&word)
    }
}
