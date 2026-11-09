//! 拼接备选：最优路径在某个词界之前换一条来路、之后照旧，得到只在一处不同的整句。
//!
//! 末尾那批节点回溯出的 n-best 多半只差最后一两个词（长句尾巴上的 啊 / 阿 / 吖），句子中间拿不准的地方没有备选，
//! 选词模型重排时也就无从改起。这里沿最优路径的每个词界，把落在那里的其他节点接上最优路径的后半段。

use super::{Context, Conversion, LanguageModel, Node, Personal, backtrack, predecessor_context};
use crate::sentence::transition_log_prob;

/// 每个词界最多接几个节点（按接上之后的得分取最好的）。
const PER_BOUNDARY: usize = 3;

/// 沿 `nodes[n][0]` 那条最优路径拼出的备选，至多 `k` 条，按得分降序，文本可能与 n-best 重复（调用方去重）。
///
/// 接口后面两个词的转移按新的上下文重算（个人三元看前两个词，再往后的不受影响），得分与真走这条路径一致。
pub(super) fn spliced(
    nodes: &[Vec<Node>],
    n: usize,
    k: usize,
    model: &dyn LanguageModel,
    personal: Personal<'_>,
) -> Vec<Conversion> {
    // 最优路径上各词的 (终点, 下标)，从后往前
    let mut chain = Vec::new();
    let (mut position, mut index) = (n, 0);
    while position > 0 {
        chain.push((position, index));
        let node = &nodes[position][index];
        (position, index) = (node.start, node.back);
    }
    let best = &nodes[n][0];
    let transition = |previous: &str, earlier: Option<&str>, node: &Node| {
        let context = Context {
            previous: Some(previous),
            earlier,
        };
        transition_log_prob(model, personal, context, &node.text, node.fallback)
    };
    let mut picks: Vec<(f64, f64, f64, usize, usize)> = Vec::new();
    for (j, pair) in chain.windows(2).enumerate() {
        let [(end, next_index), (boundary, base_index)] = [pair[0], pair[1]];
        let next = &nodes[end][next_index];
        // 再后一个词（有的话），它的三元上下文里有接口前那个词
        let after = j.checked_sub(1).map(|i| &nodes[chain[i].0][chain[i].1]);
        let base = &nodes[boundary][base_index];
        // 换掉接口前的词之后，接口后两个词的转移（动态与静态）
        let steps = |index: usize| {
            let alt = &nodes[boundary][index];
            let earlier = predecessor_context(nodes, boundary, index).earlier;
            let mut dynamic = transition(&alt.text, earlier, next);
            if let Some(after) = after {
                dynamic += transition(&next.text, Some(&alt.text), after);
            }
            let fixed = model
                .log_prob(Some(&alt.text), &next.text)
                .unwrap_or(next.fallback);
            (dynamic, fixed)
        };
        let (old, old_static) = steps(base_index);
        let mut here: Vec<_> = nodes[boundary]
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != base_index)
            .map(|(index, alt)| {
                let (dynamic, fixed) = steps(index);
                let score = best.score + alt.score - base.score + dynamic - old;
                let static_score =
                    best.static_score + alt.static_score - base.static_score + fixed - old_static;
                let penalty = best.penalty + alt.penalty - base.penalty;
                (score, static_score, penalty, boundary, index)
            })
            .collect();
        here.sort_by(|a, b| b.0.total_cmp(&a.0));
        here.truncate(PER_BOUNDARY);
        picks.extend(here);
    }
    picks.sort_by(|a, b| b.0.total_cmp(&a.0));
    picks.truncate(k);
    let whole = backtrack(nodes, n, 0);
    picks
        .into_iter()
        .map(|(score, static_score, penalty, boundary, index)| {
            let mut words = backtrack(nodes, boundary, index).words;
            let mut covered = 0;
            words.extend(
                whole
                    .words
                    .iter()
                    .filter(|word| {
                        let keep = covered >= boundary;
                        covered += word.syllables.len();
                        keep
                    })
                    .cloned(),
            );
            let text = words.iter().map(|w| w.text.as_str()).collect();
            let syllables = words
                .iter()
                .flat_map(|w| w.syllables.iter().cloned())
                .collect();
            Conversion {
                text,
                syllables,
                words,
                score,
                static_score,
                penalty,
            }
        })
        .collect()
}
