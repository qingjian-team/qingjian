//! 整句重排的学习回归数据。

use crate::candidate::Candidate;
use crate::engine::Learner;
use crate::sentence::{Context, UserNgram};

pub(super) struct Learned {
    pub(super) ngram: UserNgram,
    pub(super) chosen: Option<(&'static str, &'static str)>,
}

impl Learned {
    pub(super) fn biased() -> Self {
        let mut ngram = UserNgram::default();
        ngram.record_times(Context::START, "是", 100);
        ngram.record_times(Context::after("是"), "一", 10);
        ngram.record_times(Context::after("一"), "是", 10);
        Self {
            ngram,
            chosen: None,
        }
    }
}

impl Learner for Learned {
    fn record(&mut self, _candidate: &Candidate) {}

    fn weight(&self, text: &str) -> u32 {
        self.ngram.count(text)
    }

    fn user_ngram(&self) -> Option<&UserNgram> {
        Some(&self.ngram)
    }

    fn choice_weight(&self, input: &str, text: &str) -> u32 {
        if self.chosen == Some((input, text)) {
            20
        } else {
            0
        }
    }
}
