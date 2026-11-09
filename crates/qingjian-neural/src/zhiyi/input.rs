use super::ChoiceScorer;
use crate::NeuralError;

const CLS: u32 = 2;
const SEP: u32 = 1;
const MASK: u32 = 4;

/// 上文最多留最后这么多个 token。
const CONTEXT_TOKENS: usize = 128;

/// 每个候选最多这么多个 token。
const CANDIDATE_TOKENS: usize = 16;

/// 拼音单元最多这么多个。
const MAX_UNITS: usize = 24;

/// 一次打分的输入序列。
pub(super) struct Sequence {
    pub ids: Vec<u32>,
    pub units: Vec<u32>,
    pub markers: Vec<u32>,
}

impl ChoiceScorer {
    /// `<bos> 上文 <eos> <mask>×单元 <eos> <mask> 候选1 <mask> 候选2 … <eos>`。
    pub(super) fn sequence(
        &self,
        context: &str,
        typed: &[u32],
        candidates: &[&str],
    ) -> Result<Sequence, NeuralError> {
        let mut ids = vec![CLS];
        let context = self.encode(context);
        ids.extend(&context[context.len().saturating_sub(CONTEXT_TOKENS)..]);
        ids.push(SEP);
        let mut units = vec![0; ids.len()];
        for &unit in typed.iter().take(MAX_UNITS) {
            ids.push(MASK);
            units.push(unit);
        }
        ids.push(SEP);
        let mut markers = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            markers.push(ids.len() as u32);
            ids.push(MASK);
            let tokens = self.encode(&format!(" {candidate}"));
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
