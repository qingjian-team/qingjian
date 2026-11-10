//! 一段拼音选完后的整段记忆与撤销。

use super::{BufferPhrase, Choice};
use crate::engine::{AUTO_WORD_THRESHOLD_SAME_BUFFER, reading_key};
use crate::{Candidate, CandidateKind, Engine, Learner};

impl Engine {
    /// 一段拼音上屏完了：分几次选完（`jidiaole` 先选 挤、剩下的走整句 掉了），或一次选中由几个词组成的整句
    /// （`jiaoyizhongxin` 的 交易中心）。这几个词合起来就是用户对这段拼音的答案。
    /// 记一次「整段拼音 → 合成词」的选择；选到 [`AUTO_WORD_THRESHOLD_SAME_BUFFER`] 次、词库里没有、
    /// 不超过 `max_chars` 字就记成用户词，下次整段或它的简拼打出来直接排第一（整句也是：
    /// `xiaomingqulexuexiao` 逐字选出 晓铭去了学校，之后 `xmqlxx` 也出它）。返回记下的选择，撤销时退回。
    pub(in crate::engine) fn finish_buffer(&mut self, max_chars: usize) -> Option<BufferPhrase> {
        let words = self.chain.buffer_words();
        if words.len() < 2 {
            return None;
        }
        let text: String = words.iter().map(|(t, _)| t.as_str()).collect();
        let syllables: Vec<String> = words.iter().flat_map(|(_, s)| s.iter().cloned()).collect();
        let chars = text.chars().count();
        let key = self.chain.buffer_key().to_owned();
        if key.is_empty() || chars > max_chars || chars != syllables.len() {
            return None;
        }
        let candidate = Candidate {
            text,
            kind: CandidateKind::Chinese,
            syllables,
            reading: None,
            translation: None,
            aux_code: None,
        };
        let reading = reading_key(candidate.syllables.iter().map(String::as_str));
        self.learner.record_choice(&key, &reading, &candidate.text);
        let abbreviation = self.abbreviation_key(&candidate.syllables, &key);
        if !abbreviation.is_empty() {
            self.learner
                .record_choice(&abbreviation, "", &candidate.text);
        }
        let created = !self.knows_word(&candidate)
            && self.learner.choice_weight(&key, &candidate.text) >= AUTO_WORD_THRESHOLD_SAME_BUFFER;
        if created {
            tracing::debug!(text = %candidate.text, "buffer phrase learned as a user word");
            self.learner
                .learn_word(&candidate.text, &candidate.syllables);
            // 逐字选出来就是用户要的这个词：算选过一次，排序与整句词图里都有它的选择加分
            self.learner.record(&candidate);
        }
        Some(BufferPhrase {
            key,
            choice: Choice {
                reading,
                text: candidate.text,
                abbreviation,
            },
            created,
        })
    }
}
