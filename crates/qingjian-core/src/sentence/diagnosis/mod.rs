//! 正确答案丢在哪一步：把原句强制对齐到同一张词图上（同样的格子、同样的打分，只是不剪枝、不限束宽），
//! 分清「词库里没有 / 格子剪掉 / 束搜索挤掉 / 打分输了」。只给 CLI 评测用，不进输入路径。

mod loss;
mod parts;

use std::collections::HashMap;

use qingjian_dictionary::SyllablePattern;

pub use loss::Loss;
pub use parts::Parts;

use super::kept_hits;
use super::viterbi::{span_candidates, span_limit};
use super::{Context, MAX_WORD_SYLLABLES, SearchInput, fallback_log_prob, transition_log_prob};
use crate::ranking::weight_bonus;

/// 对齐到原句某处的一条部分路径。
struct Partial {
    parts: Parts,

    /// 前前词（前词是 map 的键）。
    earlier: Option<String>,

    /// 路径上格子名次最靠后的词：(词, 名次从 1 起, 那个格子只留几个)。
    worst: Option<(String, usize, usize)>,
}

/// 原句 `gold` 在这张词图上丢在哪。`input` 与 `positions` 同 [`super::convert_paths`]（格子名额也按 `input.limits`），
/// `best` 是词图选出的最优路径的文本。
/// 返回 [`Loss::Unreachable`] / [`Loss::Pruned`] / [`Loss::Search`] / [`Loss::Scoring`] 之一；切分与组装由调用方判断。
pub fn diagnose<W, C>(
    input: &SearchInput<'_, W, C>,
    positions: &[Vec<SyllablePattern<'_>>],
    gold: &str,
    best: &str,
) -> Loss
where
    W: Fn(&str) -> u32,
    C: Fn(usize, &str) -> f64,
{
    let align = |text: &str| align(input, positions, text);
    let path = match align(gold) {
        Ok(path) => path,
        Err(covered) => return Loss::Unreachable { covered },
    };
    if let Some((word, rank, limit)) = path.worst.filter(|w| w.1 > w.2) {
        return Loss::Pruned { word, rank, limit };
    }
    let best = align(best).map_or(
        Parts {
            score: f64::NEG_INFINITY,
            ..Parts::default()
        },
        |p| p.parts,
    );
    if path.parts.score > best.score + 1e-9 {
        Loss::Search {
            gold: path.parts,
            best,
        }
    } else {
        Loss::Scoring {
            gold: path.parts,
            best,
        }
    }
}

/// 把 `text` 对齐到词图上，取最好的一条（先要求词都在格子里，再比分数）；对不上时返回拼得出的字数。
fn align<W, C>(
    input: &SearchInput<'_, W, C>,
    positions: &[Vec<SyllablePattern<'_>>],
    gold: &str,
) -> Result<Partial, usize>
where
    W: Fn(&str) -> u32,
    C: Fn(usize, &str) -> f64,
{
    let chars: Vec<char> = gold.chars().collect();
    let n = positions.len();
    let log_total = input
        .dictionaries
        .iter()
        .map(|d| d.total_frequency() as f64)
        .sum::<f64>()
        .max(1.0)
        .ln();
    // states[(音节位置, 字位置)]：前词 → 最好的那条部分路径
    let mut states: HashMap<(usize, usize), HashMap<String, Partial>> = HashMap::new();
    states.entry((0, 0)).or_default().insert(
        String::new(),
        Partial {
            parts: Parts::default(),
            earlier: None,
            worst: None,
        },
    );
    let mut covered = 0;
    for start in 0..n {
        let keys: Vec<(usize, usize)> = states
            .keys()
            .filter(|(position, _)| *position == start)
            .copied()
            .collect();
        if keys.is_empty() {
            continue;
        }
        let spans: Vec<_> = (start + 1..=n.min(start + MAX_WORD_SYLLABLES))
            .map(|end| {
                let span = &positions[start..end];
                let hits = span_candidates(
                    input,
                    span,
                    start,
                    input.tones.get(start..end).unwrap_or(&[]),
                    usize::MAX,
                );
                let limit = span_limit(span, &input.limits);
                // 名额外按出字分收进来的词也算在格子里：名次记成名额以内
                let kept: std::collections::HashSet<usize> =
                    kept_hits(&hits, limit, start, input.emission)
                        .into_iter()
                        .map(|(rank, _)| rank)
                        .collect();
                (end, limit, hits, kept)
            })
            .collect();
        for key in keys {
            let offset = key.1;
            covered = covered.max(offset);
            let Some(here) = states.remove(&key) else {
                continue;
            };
            for (end, limit, hits, kept) in &spans {
                let (end, limit) = (*end, *limit);
                for (index, hit) in hits.iter().enumerate() {
                    let rank = if kept.contains(&index) {
                        index.min(limit - 1)
                    } else {
                        index
                    };
                    let length = hit.text.chars().count();
                    if offset + length > chars.len()
                        || !hit
                            .text
                            .chars()
                            .eq(chars[offset..offset + length].iter().copied())
                    {
                        continue;
                    }
                    let fallback = fallback_log_prob(hit.frequency, hit.secondary, log_total);
                    let emitted = (input.emission)(start, &hit.text, &hit.syllables);
                    let step = weight_bonus((input.weight)(&hit.text))
                        - hit.penalty
                        - hit.tone
                        - hit.reading
                        + emitted;
                    let next = states.entry((end, offset + length)).or_default();
                    for (previous, partial) in &here {
                        let context = if start == 0 {
                            Context::START
                        } else {
                            Context {
                                previous: Some(previous.as_str()),
                                earlier: partial.earlier.as_deref(),
                            }
                        };
                        let transition = transition_log_prob(
                            input.model,
                            input.personal,
                            context,
                            &hit.text,
                            fallback,
                        );
                        let parts = Parts {
                            score: partial.parts.score + transition + step,
                            language_model: partial.parts.language_model + transition,
                            emission: partial.parts.emission + emitted,
                        };
                        let score = parts.score;
                        let worst = match &partial.worst {
                            Some(w)
                                if w.1.saturating_sub(w.2) >= (rank + 1).saturating_sub(limit) =>
                            {
                                Some(w.clone())
                            }
                            _ => Some((hit.text.clone(), rank + 1, limit)),
                        };
                        let better = next.get(&hit.text).is_none_or(|p| {
                            // 先比是不是都在格子里（名次超限少的优先），再比分数
                            let over = |w: &Option<(String, usize, usize)>| {
                                w.as_ref().map_or(0, |w| w.1.saturating_sub(w.2))
                            };
                            (over(&worst), -score) < (over(&p.worst), -p.parts.score)
                        });
                        if better {
                            next.insert(
                                hit.text.clone(),
                                Partial {
                                    parts,
                                    earlier: (start > 0).then(|| previous.clone()),
                                    worst,
                                },
                            );
                        }
                    }
                }
            }
        }
    }
    let finals = states.remove(&(n, chars.len())).ok_or(covered)?;
    finals
        .into_values()
        .min_by(|a, b| {
            let over = |p: &Partial| p.worst.as_ref().map_or(0, |w| w.1.saturating_sub(w.2));
            (over(a), -a.parts.score)
                .partial_cmp(&(over(b), -b.parts.score))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .ok_or(covered)
}
