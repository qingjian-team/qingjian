//! 候选条上看得见的候选告诉 Core，生词本的「看到」次数靠它（同 Linux Server 按候选窗回执记）。

use qingjian_core::{Candidate, Translation};

use super::Session;
use crate::strip::StripHit;

impl Session {
    /// `hits` 是这一帧画出来、落在视口里的可点区域：候选只算看得见的，译文只留画出来的那几条。
    /// 候选条横向滚开后才露出来的不算，同桌面端只算当前一页。
    pub fn note_displayed(&mut self, hits: &[&StripHit]) {
        let candidates: Vec<Candidate> = self
            .candidates
            .items
            .iter()
            .enumerate()
            .filter(|(index, _)| {
                hits.iter()
                    .any(|hit| hit.candidate == *index && hit.sense.is_none())
            })
            .map(|(index, candidate)| {
                let mut candidate = candidate.clone();
                candidate.translation = candidate.translation.take().and_then(|translation| {
                    let senses: Vec<_> = translation
                        .senses()
                        .iter()
                        .enumerate()
                        .filter(|(sense, _)| {
                            hits.iter()
                                .any(|hit| hit.candidate == index && hit.sense == Some(*sense))
                        })
                        .map(|(_, sense)| sense.clone())
                        .collect();
                    (!senses.is_empty()).then(|| Translation::new(translation.language, senses))
                });
                candidate
            })
            .collect();
        self.engine.note_displayed(candidates.iter());
    }
}
