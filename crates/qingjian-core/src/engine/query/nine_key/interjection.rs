//! 九宫格的叹词候选：拼音列选定 ng / hm / hng（或整段就是 n / m）时给 嗯 / 噷 / 哼 / 呣，见 [`nine_key::INTERJECTIONS`]。

use crate::candidate::{Candidate, CandidateKind};
use crate::engine::Engine;
use crate::nine_key::{self, Shape};

impl Engine {
    /// 第一个位置是选定的叹词读法时给的候选与该插的位置：ng / hm / hng 排第一；
    /// `n` / `m` 多半是简拼的声母，只在整段就是这个字母时给，排在首选后面。
    pub(super) fn nine_key_interjection(
        &self,
        keys: &str,
        shape: Option<&Shape>,
    ) -> Option<(usize, Candidate)> {
        let first = shape?.positions.first()?;
        let spelling = &keys[first.start..first.start + first.len];
        let character = nine_key::interjection(spelling)?;
        let at = if spelling.len() > 1 {
            0
        } else if keys.trim_end_matches('\'') == spelling {
            1
        } else {
            return None;
        };
        Some((
            at,
            Candidate {
                text: character.to_owned(),
                kind: CandidateKind::Chinese,
                syllables: vec![spelling.to_owned()],
                reading: None,
                translation: None,
                aux_code: None,
            },
        ))
    }
}
