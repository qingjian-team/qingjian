use super::ChoiceScorer;
use crate::NeuralError;

const CLS: u32 = 2;
pub(super) const SEP: u32 = 1;
const MASK: u32 = 4;

/// 上文最多留最后这么多个 token。
const CONTEXT_TOKENS: usize = 128;

/// 每个候选最多这么多个 token。
const CANDIDATE_TOKENS: usize = 16;

/// 拼音单元最多这么多个。
const MAX_UNITS: usize = 24;

/// 一次前向的输入序列。
pub(super) struct Sequence {
    pub ids: Vec<u32>,

    pub units: Vec<u32>,

    /// 各候选的标记位置；整句序列为空（模型说明：整句不在任何位置加候选向量）。
    pub markers: Vec<u32>,
}

impl ChoiceScorer {
    /// 旧格式：`<bos> 上文 <eos> <mask>×单元 <eos> <mask> 候选1 <mask> 候选2 … <eos>`；
    /// 紧凑格式：`<bos> 上文 <eos> <mask>×单元 <eos> 候选1 候选2 … <eos>`，标记是各候选的第一个 token。
    pub(super) fn sequence(
        &self,
        context: &str,
        typed: &[u32],
        candidates: &[&str],
    ) -> Result<Sequence, NeuralError> {
        let context = self.encode(context);
        let (mut ids, mut units) = self.lead_and_slots(
            &context[context.len().saturating_sub(CONTEXT_TOKENS)..],
            typed,
        );
        let mut markers = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            let tokens = if self.format.compact {
                self.tokenizer.encode_bare(candidate)
            } else {
                self.encode(&format!(" {candidate}"))
            };
            markers.push(ids.len() as u32);
            if !self.format.compact {
                ids.push(MASK);
            } else if tokens.is_empty() {
                return Err(NeuralError::Corrupt("empty candidate"));
            }
            ids.extend(tokens.iter().take(CANDIDATE_TOKENS));
        }
        ids.push(SEP);
        units.resize(ids.len(), 0);
        Ok(Sequence {
            ids,
            units,
            markers,
        })
    }

    /// 紧凑格式的整句序列：`<bos> 前缀+上文 <eos> <mask>×单元 <eos>`，没有候选。
    /// 前缀与上文合起来超长时留前缀自己的 token 和上文末尾，共 `compose_context_tokens` 个。
    pub(super) fn compose_sequence(&self, context: &str, typed: &[u32]) -> Sequence {
        let prefix = &self.format.context_prefix;
        let mut lead = self.encode(&format!("{prefix}{context}"));
        let limit = self.format.compose_context_tokens;
        if limit > 0 && lead.len() > limit {
            let keep = self.encode(prefix).len().min(limit);
            lead.drain(keep..lead.len() - (limit - keep));
        }
        let (ids, units) = self.lead_and_slots(&lead, typed);
        Sequence {
            ids,
            units,
            markers: Vec::new(),
        }
    }

    /// `<bos> lead <eos> <mask>×单元 <eos>` 与每个位置的单元（拼音槽位以外为 0）。
    fn lead_and_slots(&self, lead: &[u32], typed: &[u32]) -> (Vec<u32>, Vec<u32>) {
        let mut ids = Vec::with_capacity(lead.len() + typed.len() + 3);
        ids.push(CLS);
        ids.extend(lead);
        ids.push(SEP);
        let mut units = vec![0; ids.len()];
        for &unit in typed.iter().take(MAX_UNITS) {
            ids.push(MASK);
            units.push(unit);
        }
        ids.push(SEP);
        units.push(0);
        (ids, units)
    }

    /// 英文：逐个字母的单元。
    pub fn letter_units(&self, letters: &str) -> Vec<u32> {
        letters
            .chars()
            .filter_map(|c| {
                let index = self
                    .letters
                    .iter()
                    .position(|&l| l == c.to_ascii_lowercase())?;
                Some((self.units.len() + index) as u32)
            })
            .collect()
    }

    /// 候选读音对上敲的字母（`zhongg` + zhong guo → `f:zhong p:g`）：每个音节敲全了是 `f:`，只敲了开头是 `p:`，
    /// 敲的字母要正好用完。对不上（只盖住一部分拼音、双拼、纠错）返回 `None`。
    pub fn align(&self, keys: &str, syllables: &[String]) -> Option<Vec<u32>> {
        let mut rest = keys;
        let mut out = Vec::with_capacity(syllables.len());
        for syllable in syllables {
            if rest.is_empty() {
                return None;
            }
            let name = if rest.starts_with(syllable.as_str()) {
                rest = &rest[syllable.len()..];
                format!("f:{syllable}")
            } else {
                let common = rest
                    .bytes()
                    .zip(syllable.bytes())
                    .take_while(|(a, b)| a == b)
                    .count();
                if common == 0 {
                    return None;
                }
                let prefix = &rest[..common];
                rest = &rest[common..];
                format!("p:{prefix}")
            };
            out.push(*self.units.get(&name)?);
        }
        rest.is_empty().then_some(out)
    }
}
