//! 字节偏移音节 DAG 上的统一 Viterbi：多种边界切法共用一张词图与一次动态规划，
//! 按词库前缀存在性剪枝多音节扩展，不再把切分截成少数形状分别转换。

use std::collections::HashSet;

use qingjian_dictionary::{Match, SyllablePattern};

use super::super::{
    Conversion, EMITTED_SPAN_POOL, LatticeEdge, MAX_WORD_SYLLABLES, Personal, SECONDARY_PENALTY,
    SearchInput, SearchLimits, SpanCache, SpanWord, fallback_log_prob, kept_hits, reading_cost,
};
use super::{NoModel, Node, UNKNOWN_LOG_PROB, backtrack, best_predecessor};
use crate::ranking::weight_bonus;

/// 在音节 DAG 上做一次统一 bigram Viterbi，返回得分最高的整句路径。
pub fn convert_lattice<W, C>(
    input: &SearchInput<'_, W, C>,
    graph: &[Vec<LatticeEdge<'_>>],
    beam_width: usize,
    cache: &mut SpanCache,
) -> Option<Conversion>
where
    W: Fn(&str) -> u32,
    C: Fn(usize, &str) -> f64,
{
    convert_lattice_paths(input, graph, beam_width, 1, cache)
        .into_iter()
        .next()
}

/// 在音节 DAG 上做一次统一 bigram Viterbi，返回得分最高的前 `k` 条去重整句路径。
///
/// `graph` 按顶点下标 `0..end` 排列，`graph[start]` 是从 `start` 出发的单音节边（`edge.to <= end`）。
/// 从每个可达顶点按词库 `has_longer` 剪枝向外展开最多 [`MAX_WORD_SYLLABLES`] 个音节，
/// 命中的词直接在 `nodes[start] -> nodes[edge.to]` 上松弛；词库无更长词的前缀原地停下。
pub fn convert_lattice_paths<W, C>(
    input: &SearchInput<'_, W, C>,
    graph: &[Vec<LatticeEdge<'_>>],
    beam_width: usize,
    k: usize,
    cache: &mut SpanCache,
) -> Vec<Conversion>
where
    W: Fn(&str) -> u32,
    C: Fn(usize, &str) -> f64,
{
    let end = graph.len();
    if end == 0 || beam_width == 0 || k == 0 {
        return Vec::new();
    }
    let total: f64 = input
        .dictionaries
        .iter()
        .map(|d| d.total_frequency() as f64)
        .sum::<f64>()
        .max(1.0);
    let log_total = total.ln();

    let mut nodes: Vec<Vec<Node>> = (0..=end).map(|_| Vec::new()).collect();
    nodes[0].push(Node {
        start: 0,
        text: String::new(),
        syllables: Vec::new(),
        score: 0.0,
        static_score: 0.0,
        back: 0,
        placeholder: false,
        penalty: 0.0,
        fallback: 0.0,
    });

    let mut span: Vec<Vec<SyllablePattern<'_>>> = Vec::with_capacity(MAX_WORD_SYLLABLES);
    for start in 0..end {
        prune_lattice(&mut nodes[start], beam_width);
        if nodes[start].is_empty() || graph[start].is_empty() {
            continue;
        }
        let mut any = false;
        let mut ctx = ExpandContext {
            input,
            graph,
            end,
            start,
            log_total,
            cache,
            nodes: &mut nodes,
            any: &mut any,
        };
        expand(&mut ctx, start, 1, 0.0, &mut span);
        if !any && let Some(edge) = graph[start].first() {
            let text = edge.patterns.first().map_or("", |p| p.text);
            let (score, back) = best_predecessor(
                &nodes,
                start,
                text,
                &NoModel,
                Personal::NONE,
                UNKNOWN_LOG_PROB,
            );
            let penalty = nodes[start][back].penalty + edge.penalty;
            let static_score = nodes[start][back].static_score + UNKNOWN_LOG_PROB;
            nodes[edge.to].push(Node {
                start,
                text: text.to_owned(),
                syllables: vec![text.to_owned()],
                score: score - edge.penalty,
                static_score,
                back,
                placeholder: true,
                penalty,
                fallback: UNKNOWN_LOG_PROB,
            });
        }
    }
    prune_lattice(&mut nodes[end], beam_width);
    let mut paths: Vec<Conversion> = Vec::with_capacity(k.min(nodes[end].len()));
    for index in 0..nodes[end].len() {
        if paths.len() >= k {
            break;
        }
        let conversion = backtrack(&nodes, end, index);
        if !paths.iter().any(|p| p.text == conversion.text) {
            paths.push(conversion);
        }
    }
    paths
}

struct ExpandContext<'ctx, 'g, 'b, W, C> {
    input: &'ctx SearchInput<'b, W, C>,
    graph: &'g [Vec<LatticeEdge<'g>>],
    end: usize,
    start: usize,
    log_total: f64,
    cache: &'ctx mut SpanCache,
    nodes: &'ctx mut [Vec<Node>],
    any: &'ctx mut bool,
}

fn expand<'ctx, 'g, 'b, W, C>(
    ctx: &mut ExpandContext<'ctx, 'g, 'b, W, C>,
    vertex: usize,
    depth: usize,
    edge_penalty: f64,
    span: &mut Vec<Vec<SyllablePattern<'g>>>,
) where
    W: Fn(&str) -> u32,
    C: Fn(usize, &str) -> f64,
{
    let limits = ctx.input.limits;
    for edge in &ctx.graph[vertex] {
        span.push(edge.patterns.clone());
        let penalty_sum = edge_penalty + edge.penalty;
        let limit = lattice_span_limit(span, &limits);
        let (hits, has_longer) = ctx.cache.get_or_insert_step(SpanCache::key(span), || {
            span_candidates_step(ctx.input, span, limit + EMITTED_SPAN_POOL)
        });
        if !hits.is_empty() {
            *ctx.any = true;
            for (_, hit) in kept_hits(&hits, limit, ctx.start, ctx.input.emission) {
                let bonus = weight_bonus((ctx.input.weight)(&hit.text));
                let fallback = fallback_log_prob(hit.frequency, hit.secondary, ctx.log_total);
                let emitted = (ctx.input.emission)(ctx.start, &hit.text, &hit.syllables);
                let (score, back) = best_predecessor(
                    ctx.nodes,
                    ctx.start,
                    &hit.text,
                    ctx.input.model,
                    ctx.input.personal,
                    fallback,
                );
                let previous = &ctx.nodes[ctx.start][back];
                let penalty = previous.penalty + hit.penalty + penalty_sum;
                let static_step = ctx
                    .input
                    .model
                    .log_prob((ctx.start > 0).then_some(previous.text.as_str()), &hit.text)
                    .unwrap_or(fallback);
                let static_score = previous.static_score + static_step;
                ctx.nodes[edge.to].push(Node {
                    start: ctx.start,
                    text: hit.text.clone(),
                    syllables: hit.syllables.clone(),
                    score: score + bonus - hit.penalty - hit.tone - hit.reading - penalty_sum
                        + emitted,
                    static_score,
                    back,
                    placeholder: false,
                    penalty,
                    fallback,
                });
            }
        }
        if has_longer
            && depth < MAX_WORD_SYLLABLES
            && edge.to < ctx.end
            && !ctx.graph[edge.to].is_empty()
        {
            expand(ctx, edge.to, depth + 1, penalty_sum, span);
        }
        span.pop();
    }
}

/// 同 `span_candidates`，同时合并全部词库的 `has_longer`，给 DAG 词图剪枝。
fn span_candidates_step<W, C>(
    input: &SearchInput<'_, W, C>,
    span: &[Vec<SyllablePattern<'_>>],
    limit: usize,
) -> (Vec<SpanWord>, bool)
where
    W: Fn(&str) -> u32,
    C: Fn(usize, &str) -> f64,
{
    let mut has_longer = false;
    let mut scored: Vec<(f64, Match<'_>)> = Vec::new();
    for dictionary in input.dictionaries {
        let (matches, longer) = dictionary.lookup_exact_alt_step(span);
        has_longer |= longer;
        for m in matches {
            let seen = (input.weight)(m.text) + input.personal.count(m.text);
            if m.frequency == 0 && seen == 0 {
                continue;
            }
            let secondary = if m.secondary { -SECONDARY_PENALTY } else { 0.0 };
            let score =
                f64::from(m.frequency.max(1)) * (1.0 + f64::from(seen)) * (-secondary).exp();
            scored.push((score, m));
        }
    }
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.dedup_by(|a, b| a.1.text == b.1.text);
    scored.truncate(limit);
    let words = scored
        .into_iter()
        .map(|(_, hit)| SpanWord {
            text: hit.text.to_owned(),
            syllables: hit.syllables().map(str::to_owned).collect(),
            frequency: hit.frequency,
            secondary: hit.secondary,
            penalty: 0.0,
            tone: 0.0,
            reading: reading_cost(&hit),
        })
        .collect();
    (words, has_longer)
}

/// DAG 格子的候选名额：有简拼位置时走 `abbreviated_span_candidates`；完整音节格按该格覆盖的
/// 同码读法组合数等比放大（最多 3 倍），避免单码 1 种读法（如 `ding` / `xiang`）混入生僻同音字，
/// 同时让 4–5 种读法共享一格的码（如 `7426` / `98`）不至于把次高频音节挤出词图。
fn lattice_span_limit(span: &[Vec<SyllablePattern<'_>>], limits: &SearchLimits) -> usize {
    if span.iter().any(|p| p.iter().any(|t| !t.complete)) {
        return limits.abbreviated_span_candidates;
    }
    let combinations = span
        .iter()
        .map(|p| p.len().max(1))
        .fold(1usize, usize::saturating_mul)
        .min(3);
    limits.span_candidates * combinations
}

/// 按得分降序留前 `beam_width` 条，同结尾词只留最高分的一条：
/// DAG 上不同起点的边可能落在同一字节偏移上且词形相同，后续二元转移分一样，去重可省出束宽给别的词。
fn prune_lattice(nodes: &mut Vec<Node>, beam_width: usize) {
    nodes.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut seen: HashSet<String> = HashSet::with_capacity(beam_width);
    nodes.retain(|node| seen.len() < beam_width && seen.insert(node.text.clone()));
}
