//! 按词跳：首选整句的第一个词先定下来留在组句里，候选换成从下一个词起的，改长句中间的词不用挪光标。

use qingjian_dictionary::SyllablePattern;

use super::super::Engine;
use crate::candidate::{Candidate, CandidateKind};
use crate::sentence::SentenceWord;

/// 按词库切首词时最长试几个字：词库里超过 8 字的词极少，再长的多是整句。
const MAX_WORD_CHARS: usize = 8;

impl Engine {
    /// 把首选整句 `lead` 的第一个词当作选中，留在组句里（同手动分几次选词：退格撤回、整段选完一起上屏）。
    /// `lead` 是壳手上当前候选的第一个。不是由几个词拼成的整句（单个词、云端整句、切不出首词的）、
    /// 或关了选词后留在组句里（[`Self::set_hold_selections`]）时不动，返回 `None`。
    /// 动了返回要交给应用的文本，同 [`Self::commit`]（通常是空串：词留在组句里），之后要重新 [`Self::query`]。
    pub fn hold_next_word(&mut self, lead: &Candidate) -> Option<String> {
        if !self.hold_selections || lead.kind != CandidateKind::Sentence {
            return None;
        }
        // 繁体输出开着时候选是繁体，整句的词序列按简体对
        let simplified = self.traditional.restore(&lead.text).map(|text| Candidate {
            text,
            ..lead.clone()
        });
        // 知意重排、学过的整句与本地重算的整句不一定是同一句，对不上时按词库从句首切最长的词
        let lead = simplified.as_ref().unwrap_or(lead);
        let first = match self.sentence_words(lead) {
            Some(words) if words.len() > 1 => words.into_iter().next()?,
            Some(_) => return None,
            None => self.leading_word(lead)?,
        };
        if first.placeholder {
            return None;
        }
        let word = Candidate {
            text: self.traditional.convert_remember(&first.text),
            kind: CandidateKind::Chinese,
            syllables: first.syllables.clone(),
            reading: None,
            translation: None,
            aux_code: None,
        };
        Some(self.commit(&word))
    }

    /// 整句开头词库里有的最长的词（至少留一个字在后面），都没有就取第一个字；整句的字和音节对不上时为 `None`。
    fn leading_word(&self, lead: &Candidate) -> Option<SentenceWord> {
        let chars: Vec<char> = lead.text.chars().collect();
        if chars.len() < 2 || chars.len() != lead.syllables.len() {
            return None;
        }
        let longest = (chars.len() - 1).min(MAX_WORD_CHARS);
        let len = (2..=longest)
            .rev()
            .find(|&len| {
                let text: String = chars[..len].iter().collect();
                let pattern: Vec<SyllablePattern<'_>> = lead.syllables[..len]
                    .iter()
                    .map(|text| SyllablePattern {
                        text,
                        complete: true,
                    })
                    .collect();
                self.all_dictionaries()
                    .into_iter()
                    .any(|d| d.lookup_exact(&pattern).iter().any(|hit| hit.text == text))
            })
            .unwrap_or(1);
        Some(SentenceWord {
            text: chars[..len].iter().collect(),
            syllables: lead.syllables[..len].to_vec(),
            placeholder: false,
        })
    }
}
