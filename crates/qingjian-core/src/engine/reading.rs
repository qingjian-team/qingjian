//! 拼音行上的带调拼音（`[general] show_tones`）：声调先按词库里的词取，词库没标的字按单字声调表补。

use qingjian_dictionary::SyllablePattern;

use super::Engine;
use crate::candidate::{Candidate, CandidateKind};
use crate::zhuyin::tone;

/// 整句按词库切词时最长试几个字。
const MAX_WORD_CHARS: usize = 8;

impl Engine {
    /// 候选逐音节的带调拼音（`["gān", "gà"]`），给拼音行用（[`crate::Query::window_segments`]）。
    /// 开关关着、注音与九宫格（拼音行不是拼音）、不是中文 / 整句 / 云端词、字数与音节数对不上的不给。
    pub fn toned_syllables(&self, candidate: &Candidate) -> Option<Vec<String>> {
        if !self.show_tones
            || self.zhuyin
            || self.nine_key
            || !matches!(
                candidate.kind,
                CandidateKind::Chinese | CandidateKind::Sentence | CandidateKind::Cloud
            )
        {
            return None;
        }
        // 繁体输出开着时按简体原文查：词库是简体
        let text = self
            .traditional
            .restore(&candidate.text)
            .unwrap_or_else(|| candidate.text.clone());
        self.toned_reading(&text, &candidate.syllables)
    }

    /// `text` 按 `syllables` 读的带调拼音。整句没有词边界，从左往右取词库里最长的词；单字词库不带声调，直接查单字表。
    fn toned_reading(&self, text: &str, syllables: &[String]) -> Option<Vec<String>> {
        let chars: Vec<char> = text.chars().collect();
        if chars.is_empty() || chars.len() != syllables.len() {
            return None;
        }
        let mut tones = vec![0; chars.len()];
        let mut start = 0;
        while start < chars.len() {
            let longest = (start + MAX_WORD_CHARS).min(chars.len());
            let found = (start + 2..=longest).rev().find_map(|end| {
                let word: String = chars[start..end].iter().collect();
                self.word_tones(&word, &syllables[start..end])
                    .map(|found| (end, found))
            });
            let end = match found {
                Some((end, found)) => {
                    tones[start..end].copy_from_slice(&found);
                    end
                }
                None => start + 1,
            };
            for index in start..end {
                if tones[index] == 0 {
                    tones[index] = tone::usual(chars[index], &syllables[index]);
                }
            }
            start = end;
        }
        Some(
            syllables
                .iter()
                .zip(tones)
                .map(|(syllable, tone)| qingjian_dictionary::marked(syllable, tone))
                .collect(),
        )
    }

    /// 词库里按这串音节读的 `word` 的声调：有带声调的那条就用它，只有不带声调的给全 0，查不到是 `None`。
    fn word_tones(&self, word: &str, syllables: &[String]) -> Option<Vec<u8>> {
        let patterns: Vec<_> = syllables
            .iter()
            .map(|s| SyllablePattern::complete(s))
            .collect();
        let mut found = None;
        for dictionary in self.all_dictionaries() {
            for hit in dictionary.lookup_exact(&patterns) {
                if hit.text != word {
                    continue;
                }
                let tones = hit.tones.to_vec(syllables.len());
                if tones.iter().any(|&t| t != 0) {
                    return Some(tones);
                }
                found = Some(tones);
            }
        }
        found
    }
}
