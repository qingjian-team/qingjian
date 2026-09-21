//! 记录原样读取前后学习接口调用，保证只读路径连查询都不触发。
use crate::{Candidate, Learner};
use std::sync::{Arc, Mutex};

pub(super) struct ObservedLearner(pub Arc<Mutex<Vec<String>>>);

impl Learner for ObservedLearner {
    fn record(&mut self, _: &Candidate) {
        self.0.lock().unwrap().push("record".into());
    }

    fn weight(&self, _: &str) -> u32 {
        self.0.lock().unwrap().push("weight".into());
        0
    }

    fn record_raw(&mut self, _: &str) {
        self.0.lock().unwrap().push("raw".into());
    }

    fn learn_english(&mut self, _: &str) {
        self.0.lock().unwrap().push("english".into());
    }
}
