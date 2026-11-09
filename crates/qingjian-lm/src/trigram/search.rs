//! 给文字切词打分：按字走一遍，每个位置留最好的几条（最近两个词不同的）部分路径。
//! 一段候选的分是「上文 + 候选」的最优路径减去上文自己的最优路径，即 ln P(候选 | 上文)。

use super::TrigramModel;
use super::state::State;

/// 上文只看最后一行的最后这么多个字：三元模型看不了更远，多了只是多算。
const CONTEXT_CHARS: usize = 12;

/// 一个词最多几个字。
const MAX_WORD_CHARS: usize = 8;

/// 每个位置留几条部分路径。
const BEAM: usize = 8;

const LN_10: f64 = std::f64::consts::LN_10;

impl TrigramModel {
    /// 每段 `texts` 接在 `context` 后面的 ln P；模型一个字都切不出来时是负无穷。
    pub fn score_texts(&self, context: &str, texts: &[&str]) -> Vec<f64> {
        let line = context.rsplit(['\n', '\r']).next().unwrap_or_default();
        let context: Vec<&str> = graphemes(line);
        let context = &context[context.len().saturating_sub(CONTEXT_CHARS)..];
        let mut top = if context.is_empty() {
            vec![State::START]
        } else {
            self.run(context, 0, vec![State::START])
        };
        keep_best(&mut top);
        let Some(base) = top.first().map(|s| s.log10) else {
            return vec![f64::NEG_INFINITY; texts.len()];
        };
        texts
            .iter()
            .map(|text| {
                let mut chars = context.to_vec();
                chars.extend(graphemes(text));
                let best = self
                    .run(&chars, context.len(), top.clone())
                    .iter()
                    .map(|s| s.log10)
                    .fold(f64::NEG_INFINITY, f64::max);
                if best == f64::NEG_INFINITY {
                    best
                } else {
                    LN_10 * (best - base)
                }
            })
            .collect()
    }

    /// 从 `start` 处的 `states` 起把 `chars[start..]` 切完，返回走到末尾的各条路径。
    /// 每个位置试 1 到 [`MAX_WORD_CHARS`] 个字的词，单字不在词表里就当 `<unk>`。
    fn run(&self, chars: &[&str], start: usize, states: Vec<State>) -> Vec<State> {
        let n = chars.len();
        let mut at: Vec<Vec<State>> = vec![Vec::new(); n + 1];
        at[start] = states;
        let mut text = String::new();
        for i in start..n {
            let mut here = std::mem::take(&mut at[i]);
            if here.is_empty() {
                continue;
            }
            keep_best(&mut here);
            let mut edges: Vec<(usize, u32)> = Vec::new();
            text.clear();
            for len in 1..=MAX_WORD_CHARS.min(n - i) {
                text.push_str(chars[i + len - 1]);
                if let Some(word) = self.word(&text) {
                    edges.push((i + len, word));
                }
            }
            if !edges.iter().any(|&(end, _)| end == i + 1) {
                edges.push((i + 1, self.unknown()));
            }
            for state in &here {
                for &(end, word) in &edges {
                    let log10 = state.log10 + self.log10(state.previous, state.last, word);
                    let there = &mut at[end];
                    // 最近两个词相同的路径以后的走法一样，只留分高的
                    match there
                        .iter_mut()
                        .find(|o| o.previous == state.last && o.last == Some(word))
                    {
                        Some(existing) if existing.log10 >= log10 => {}
                        Some(existing) => existing.log10 = log10,
                        None => there.push(State {
                            previous: state.last,
                            last: Some(word),
                            log10,
                        }),
                    }
                }
            }
            at[i] = here;
        }
        std::mem::take(&mut at[n])
    }
}

/// 按分降序只留前 [`BEAM`] 条；同分保持先到的在前。
fn keep_best(states: &mut Vec<State>) {
    states.sort_by(|a, b| b.log10.total_cmp(&a.log10));
    states.truncate(BEAM);
}

/// 按字（Unicode 标量）切开。
fn graphemes(text: &str) -> Vec<&str> {
    text.char_indices()
        .map(|(i, c)| &text[i..i + c.len_utf8()])
        .collect()
}
