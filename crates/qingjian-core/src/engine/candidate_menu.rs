//! 候选右键菜单背后的操作：置顶要的输入码与词、降权与取消降权，以及查询时按降权往后挪。

use super::{Engine, Learner};
use crate::candidate::{Candidate, CandidateKind};

impl Engine {
    /// 这个候选存进学习数据与自定义短语时用的写法：繁体输出开着时换回简体。
    fn stored_text(&self, candidate: &Candidate) -> String {
        self.traditional
            .restore(&candidate.text)
            .unwrap_or_else(|| candidate.text.clone())
    }

    /// 把这个候选置顶要建的自定义短语：(输入码, 词)。输入码就是当前作用域的字母；
    /// 英文模式、作用域不是合法输入码（带数字、分隔符或太长）时给 `None`，壳不出「置顶」。
    pub fn pin_target(&self, candidate: &Candidate) -> Option<(String, String)> {
        let code = self.composition.scope();
        (!self.english_mode && crate::custom_phrase::valid_code(code))
            .then(|| (code.to_owned(), self.stored_text(candidate)))
    }

    /// 降得了权的：自定义短语按权重排，不走降权。
    pub fn demotable(&self, candidate: &Candidate) -> bool {
        !matches!(candidate.kind, CandidateKind::Custom(_)) && !self.composition.scope().is_empty()
    }

    /// 在当前输入串下把这个候选降权一次（往后挪一格）。私密输入、关了学习时照做。
    pub fn demote(&mut self, candidate: &Candidate) {
        let text = self.stored_text(candidate);
        let input = self.composition.scope().to_owned();
        self.learner.demote(&input, &text);
    }

    /// 取消当前输入串下这个候选的降权。返回原来有没有降过。
    pub fn undemote(&mut self, candidate: &Candidate) -> bool {
        let text = self.stored_text(candidate);
        let input = self.composition.scope().to_owned();
        self.learner.undemote(&input, &text)
    }

    /// 当前输入串下这个候选降了几次。
    pub fn demotion(&self, candidate: &Candidate) -> u32 {
        self.learner
            .demotion(self.composition.scope(), &self.stored_text(candidate))
    }

    /// 排好的候选里降过权的往后挪：降了 `n` 次就排到原来在它后面第 `n` 个候选之后。
    pub(super) fn apply_demotions(&self, items: &mut Vec<Candidate>) {
        let input = self.composition.scope();
        let mut keyed: Vec<_> = items
            .drain(..)
            .enumerate()
            .map(|(index, candidate)| {
                let demotion = self.learner.demotion(input, &candidate.text) as usize;
                ((index + demotion, demotion > 0), candidate)
            })
            .collect();
        keyed.sort_by_key(|(key, _)| *key);
        items.extend(keyed.into_iter().map(|(_, candidate)| candidate));
    }
}
