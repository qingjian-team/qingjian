//! 格子里哪些词进词图：按词频的名额，加上出字分看好的几个。

use super::word::SpanWord;
use crate::sentence::{EMITTED_SPAN_EXTRAS, EMITTED_SPAN_POOL};

/// 格子里进词图的词与它按词频的名次（从 0 起）：前 `limit` 个，再加名额之后出字分比这几个都高的至多
/// [`EMITTED_SPAN_EXTRAS`] 个（出字分高的在前）。`hits` 按词频排好、至少 `limit` 个之后还带着 [`EMITTED_SPAN_POOL`] 个备选。
/// 出字分随上文变，格子缓存存的是整个备选池，每次查询现挑。
pub fn kept_hits<'h>(
    hits: &'h [SpanWord],
    limit: usize,
    start: usize,
    emission: &dyn Fn(usize, &str, &[String]) -> f64,
) -> Vec<(usize, &'h SpanWord)> {
    let head = hits.len().min(limit);
    let mut kept: Vec<(usize, &SpanWord)> = hits[..head].iter().enumerate().collect();
    let pool = &hits[head..hits.len().min(limit + EMITTED_SPAN_POOL)];
    if pool.is_empty() {
        return kept;
    }
    let best = kept
        .iter()
        .map(|(_, hit)| emission(start, &hit.text, &hit.syllables))
        .fold(f64::NEG_INFINITY, f64::max);
    let mut extras: Vec<(f64, usize, &SpanWord)> = pool
        .iter()
        .enumerate()
        .map(|(index, hit)| {
            (
                emission(start, &hit.text, &hit.syllables),
                head + index,
                hit,
            )
        })
        .filter(|(emitted, _, _)| *emitted > best)
        .collect();
    extras.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    kept.extend(
        extras
            .into_iter()
            .take(EMITTED_SPAN_EXTRAS)
            .map(|(_, rank, hit)| (rank, hit)),
    );
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(text: &str) -> SpanWord {
        SpanWord {
            text: text.to_owned(),
            syllables: vec!["ba".to_owned()],
            frequency: 1,
            secondary: false,
            penalty: 0.0,
            tone: 0.0,
            reading: 0.0,
        }
    }

    fn texts(kept: &[(usize, &SpanWord)]) -> Vec<String> {
        kept.iter().map(|(_, w)| w.text.clone()).collect()
    }

    #[test]
    fn words_the_model_rates_above_the_kept_ones_join_the_lattice() {
        let hits: Vec<SpanWord> = ["把", "吧", "八", "爸", "拔", "霸", "罢", "坝"]
            .iter()
            .map(|t| word(t))
            .collect();
        // 名额 3：把 吧 八；霸 罢 的出字分比这三个都高，坝 不比它们高
        let emission = |_: usize, text: &str, _: &[String]| match text {
            "霸" => -0.1,
            "罢" => -0.5,
            "把" => -1.0,
            _ => -6.0,
        };
        let kept = kept_hits(&hits, 3, 0, &emission);
        assert_eq!(texts(&kept), ["把", "吧", "八", "霸", "罢"]);
        assert_eq!(kept[3].0, 5);
    }

    #[test]
    fn without_a_model_only_the_frequency_slots_are_kept() {
        let hits: Vec<SpanWord> = ["把", "吧", "八", "爸"].iter().map(|t| word(t)).collect();
        let kept = kept_hits(&hits, 2, 0, &|_, _, _| 0.0);
        assert_eq!(texts(&kept), ["把", "吧"]);
    }

    #[test]
    fn at_most_a_few_extras_join() {
        let hits: Vec<SpanWord> = (0..12).map(|i| word(&i.to_string())).collect();
        let emission = |_: usize, text: &str, _: &[String]| {
            if text.parse::<usize>().unwrap() < 2 {
                -9.0
            } else {
                0.0
            }
        };
        assert_eq!(
            kept_hits(&hits, 2, 0, &emission).len(),
            2 + EMITTED_SPAN_EXTRAS
        );
    }
}
