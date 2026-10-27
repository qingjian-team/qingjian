//! 词图上的最优路径：bigram Viterbi + 束搜索。
//!
//! 状态只按前一个词分（束宽内），个人三元要的前二词取前驱节点的回指（它那条最优路径上的前一个词）：
//! 不扩状态，代价是三元上下文是近似的，个人数据量下够用。

use qingjian_dictionary::{Match, SyllablePattern};

use super::{
    Context, Conversion, LanguageModel, MAX_WORD_SYLLABLES, MIN_PARTIAL_LETTERS, Personal,
    SECONDARY_PENALTY, SearchInput, SentenceWord, SpanCache, SpanWord, TONE_PENALTY,
    fallback_log_prob, reading_cost, transition_log_prob,
};
use crate::ranking::weight_bonus;
use crate::zhuyin::tone;

/// 词库里没有的孤立音节（罕见音节没有单字）按这个 log 概率兜底，让路径总能走通。
const UNKNOWN_LOG_PROB: f64 = -30.0;

/// 一条部分路径的末尾节点。
struct Node {
    /// 这个词从第几个音节开始。
    start: usize,

    /// 词。
    text: String,

    /// 词的音节。
    syllables: Vec<String>,

    /// 到此为止的累计得分。
    score: f64,

    /// 累计得分里静态模型的部分（见 `Conversion::static_score`）。
    static_score: f64,

    /// 前驱在 `nodes[start]` 里的下标；`start == 0` 时无意义。
    back: usize,

    /// 是占位音节。
    placeholder: bool,

    /// 到此为止路径上模糊音 / 敲错变体的代价之和（已从 `score` 里扣掉，另记一份给调用方判断路径是不是原样）。
    penalty: f64,
}

/// 把音节序列转成最可能的词序列。`positions` 每个位置是若干写法（第一种是敲的，其余是模糊音 / 敲错变体），
/// 打分用的词库、模型、个人数据与搜索规模在 `input` 里；`cache` 是格子候选的缓存
/// （调用方保证它与词库、`weight`、`personal`、`cost` 一致，这些一变就清）。
///
/// 简拼位置（`w x q`）按前缀取词：每个格子的候选会多得多，由语言模型在路径上分辨。
/// 全拼句子末尾落单的声母也当前缀读（`youdiandongx` → 有点东西）；落单的单元音（i / u）与短于
/// 两个字母的其他前缀多半是没打完的韵母，不算它；前面已有简拼的句子里末尾单字母就是一个音节。
pub fn convert<W, C>(
    input: &SearchInput<'_, W, C>,
    positions: &[Vec<SyllablePattern<'_>>],
    cache: &mut SpanCache,
) -> Option<Conversion>
where
    W: Fn(&str) -> u32,
    C: Fn(usize, &str) -> f64,
{
    convert_paths(input, positions, false, 1, cache)
        .into_iter()
        .next()
}

/// 同 [`convert`]，但全拼句子末尾的单字母也当一个音节读（`huo z…` → 或者）：
/// 给「整段拼音读法」与别的读法比分用，比分要两边覆盖同样多的字母。
pub fn convert_whole<W, C>(
    input: &SearchInput<'_, W, C>,
    positions: &[Vec<SyllablePattern<'_>>],
    cache: &mut SpanCache,
) -> Option<Conversion>
where
    W: Fn(&str) -> u32,
    C: Fn(usize, &str) -> f64,
{
    convert_paths(input, positions, true, 1, cache)
        .into_iter()
        .next()
}

/// 得分最高的前 `k` 条路径（最多束宽条，按得分降序，文本相同的只留一条）：给重打分用。
/// `keep_partial` 为真时同 [`convert_whole`]；注音的声调按 [`TONE_PENALTY`] 给格子选词打折、给路径扣分。
pub fn convert_paths<W, C>(
    input: &SearchInput<'_, W, C>,
    positions: &[Vec<SyllablePattern<'_>>],
    keep_partial: bool,
    k: usize,
    cache: &mut SpanCache,
) -> Vec<Conversion>
where
    W: Fn(&str) -> u32,
    C: Fn(usize, &str) -> f64,
{
    let Some((last, head)) = positions.split_last() else {
        return Vec::new();
    };
    let Some(&last) = last.first() else {
        return Vec::new();
    };
    let abbreviated_head = head.iter().any(|p| p.first().is_none_or(|t| !t.complete));
    let positions = if keep_partial
        || last.complete
        || abbreviated_head
        || last.text.len() >= MIN_PARTIAL_LETTERS
        // 全拼句末尾落单的声母当简拼音节（`youdiandongx` → 有点东西）；落单的单元音（i / u）
        // 更可能是没打完的韵母，仍不算它。a / o / e 在切分器里就是完整音节，走不到这里
        || crate::parser::INITIALS.contains(&last.text)
    {
        positions
    } else {
        head
    };
    let n = positions.len();
    if n == 0 || k == 0 {
        return Vec::new();
    }
    let total: f64 = input
        .dictionaries
        .iter()
        .map(|d| d.total_frequency() as f64)
        .sum::<f64>()
        .max(1.0);
    let log_total = total.ln();

    // nodes[i]：覆盖前 i 个音节、以某个词结尾的部分路径；nodes[0] 是虚拟起点
    let mut nodes: Vec<Vec<Node>> = (0..=n).map(|_| Vec::new()).collect();
    nodes[0].push(Node {
        start: 0,
        text: String::new(),
        syllables: Vec::new(),
        score: 0.0,
        static_score: 0.0,
        back: 0,
        placeholder: false,
        penalty: 0.0,
    });
    for start in 0..n {
        prune(&mut nodes[start], input.limits.beam_width);
        if nodes[start].is_empty() {
            continue;
        }
        let mut any = false;
        for end in start + 1..=n.min(start + MAX_WORD_SYLLABLES) {
            let span = &positions[start..end];
            let span_tones = input.tones.get(start..end).unwrap_or(&[]);
            let hits = cache.get_or_insert_with(SpanCache::toned_key(span, span_tones), || {
                span_candidates(input, span, start, span_tones)
            });
            if hits.is_empty() {
                continue;
            }
            any = true;
            for hit in hits.iter() {
                let bonus = weight_bonus((input.weight)(&hit.text));
                let fallback = fallback_log_prob(hit.frequency, hit.secondary, log_total);
                let (score, back) = best_predecessor(
                    &nodes,
                    start,
                    &hit.text,
                    input.model,
                    input.personal,
                    fallback,
                );
                let previous = &nodes[start][back];
                let penalty = previous.penalty + hit.penalty;
                let static_step = input
                    .model
                    .log_prob((start > 0).then_some(previous.text.as_str()), &hit.text)
                    .unwrap_or(fallback);
                let static_score = previous.static_score + static_step;
                nodes[end].push(Node {
                    start,
                    text: hit.text.clone(),
                    syllables: hit.syllables.clone(),
                    score: score + bonus - hit.penalty - hit.tone - hit.reading,
                    static_score,
                    back,
                    placeholder: false,
                    penalty,
                });
            }
        }
        // 这个音节连单字都查不到：用音节本身占位，别让整句断掉
        if !any {
            let text = positions[start][0].text;
            let (score, back) = best_predecessor(
                &nodes,
                start,
                text,
                &NoModel,
                Personal::NONE,
                UNKNOWN_LOG_PROB,
            );
            let penalty = nodes[start][back].penalty;
            let static_score = nodes[start][back].static_score + UNKNOWN_LOG_PROB;
            nodes[start + 1].push(Node {
                start,
                text: text.to_owned(),
                syllables: vec![text.to_owned()],
                score,
                static_score,
                back,
                placeholder: true,
                penalty,
            });
        }
    }
    prune(&mut nodes[n], input.limits.beam_width);
    let mut paths: Vec<Conversion> = Vec::with_capacity(k.min(nodes[n].len()));
    for index in 0..nodes[n].len() {
        if paths.len() >= k {
            break;
        }
        let conversion = backtrack(&nodes, n, index);
        if !paths.iter().any(|p| p.text == conversion.text) {
            paths.push(conversion);
        }
    }
    paths
}

/// 从 `nodes[position][index]` 回溯出整条路径。
fn backtrack(nodes: &[Vec<Node>], mut position: usize, mut index: usize) -> Conversion {
    let score = nodes[position][index].score;
    let static_score = nodes[position][index].static_score;
    let penalty = nodes[position][index].penalty;
    let mut words: Vec<SentenceWord> = Vec::new();
    while position > 0 {
        let node = &nodes[position][index];
        words.push(SentenceWord {
            text: node.text.clone(),
            syllables: node.syllables.clone(),
            placeholder: node.placeholder,
        });
        position = node.start;
        index = node.back;
    }
    words.reverse();
    let mut text = String::new();
    let mut syllables = Vec::new();
    for word in &words {
        text.push_str(&word.text);
        syllables.extend(word.syllables.iter().cloned());
    }
    Conversion {
        text,
        syllables,
        words,
        score,
        static_score,
        penalty,
    }
}

/// 占位音节不问语言模型。
struct NoModel;

impl LanguageModel for NoModel {
    fn log_prob(&self, _previous: Option<&str>, _word: &str) -> Option<f64> {
        None
    }
}

/// 一个格子里的候选词：所有词库的精确命中，按词频（加用户选择次数与个人出现次数，替代写法命中的按代价打折）取前几个。
/// 个人次数只在这里保证用户常用的同音词进得了格子，不进路径打分（那是 n-gram 的事）；
/// 打折让敲错变体命中的词只在原样命中不够多时才进格子，而常用词（关系）即使打折也留得住。
/// 格子里有简拼位置时命中的是一大片不同读音的词，多留一些让语言模型去挑。`tones` 是这个格子那几位的声调。
fn span_candidates<W, C>(
    input: &SearchInput<'_, W, C>,
    span: &[Vec<SyllablePattern<'_>>],
    start: usize,
    tones: &[Option<u8>],
) -> Vec<SpanWord>
where
    W: Fn(&str) -> u32,
    C: Fn(usize, &str) -> f64,
{
    let alternatives = span.iter().any(|p| p.len() > 1);
    let penalty_of = |m: &Match<'_>| {
        if !alternatives {
            return 0.0;
        }
        m.syllables()
            .enumerate()
            .map(|(index, syllable)| (input.cost)(start + index, syllable))
            .sum::<f64>()
    };
    // 得分先算好再排：单字母简拼的格子能命中几千条，比较器里每次查两张表会让排序占掉十几毫秒
    // 词频 0 的生僻字不进词图，用户自己选过的除外：`keneng` 按 `ken eng` 切时有了 鞥 就会读出 肯鞥，压过敲错边读出的 可能
    let mut scored: Vec<(f64, f64, f64, Match<'_>)> = input
        .dictionaries
        .iter()
        .flat_map(|d| d.lookup_exact_alt(span))
        .filter_map(|m| {
            let seen = (input.weight)(m.text) + input.personal.count(m.text);
            if m.frequency == 0 && seen == 0 {
                return None;
            }
            let penalty = penalty_of(&m);
            let tone = f64::from(tone::cost(m.text, m.syllables(), tones)) * TONE_PENALTY;
            // 补充词库的片段多，不打折会把敲错边上的常用词（`zanen` 的 咱们）挤出格子
            let secondary = if m.secondary { -SECONDARY_PENALTY } else { 0.0 };
            let score = f64::from(m.frequency.max(1))
                * (1.0 + f64::from(seen))
                * (-(penalty + tone + secondary)).exp();
            Some((score, penalty, tone, m))
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.dedup_by(|a, b| a.3.text == b.3.text);
    let abbreviated = span.iter().any(|p| p.iter().any(|t| !t.complete));
    scored.truncate(if abbreviated {
        input.limits.abbreviated_span_candidates
    } else {
        input.limits.span_candidates
    });
    scored
        .into_iter()
        .map(|(_, penalty, tone, hit)| SpanWord {
            text: hit.text.to_owned(),
            syllables: hit.syllables().map(str::to_owned).collect(),
            frequency: hit.frequency,
            secondary: hit.secondary,
            penalty,
            tone,
            reading: reading_cost(&hit),
        })
        .collect()
}

/// 在 `nodes[start]` 的前驱里挑让 `word` 得分最高的那条，返回 (累计得分, 前驱下标)。
/// 转移概率先问静态模型（不认识就用词库兜底值），再与个人 n-gram 插值；前二词是前驱自己的前驱（回指）。
fn best_predecessor(
    nodes: &[Vec<Node>],
    start: usize,
    word: &str,
    model: &dyn LanguageModel,
    personal: Personal<'_>,
    fallback: f64,
) -> (f64, usize) {
    let mut best = (f64::NEG_INFINITY, 0);
    for (index, previous) in nodes[start].iter().enumerate() {
        let context = if start == 0 {
            Context::START
        } else {
            Context {
                previous: Some(previous.text.as_str()),
                earlier: (previous.start > 0)
                    .then(|| nodes[previous.start][previous.back].text.as_str()),
            }
        };
        let score = previous.score + transition_log_prob(model, personal, context, word, fallback);
        if score > best.0 {
            best = (score, index);
        }
    }
    best
}

/// 按得分降序只留束宽条。`beam_width` 见 [`super::SearchLimits::beam_width`]。
fn prune(nodes: &mut Vec<Node>, beam_width: usize) {
    nodes.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    nodes.truncate(beam_width);
}

#[cfg(test)]
mod tests;
