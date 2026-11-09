//! 九宫格给壳显示的两样东西：拼音列与拼音行。

use std::cmp::Reverse;

use crate::candidate::Candidate;
use crate::nine_key::{self, Position, Shape};

/// 拼音列：第一个没定下来的位置上的全部写法，按「带这个音节的最靠前候选」排序，没有候选的排最后；
/// 再接从这个位置起别的长度能读成的音节（长的在前）与单个字母，`643` 除了 nie / mie 还能选 ni、mi、n、m、o。
pub(super) fn pinyin_choices(
    keys: &str,
    shape: Option<&Shape>,
    items: &[Candidate],
) -> Vec<String> {
    let Some((index, position)) = shape.and_then(Shape::first_open) else {
        return Vec::new();
    };
    let mut choices = ranked(index, position, items);
    let mut others: Vec<String> = nine_key::spellings_from(keys, position.start)
        .into_iter()
        .filter(|spelling| !choices.contains(spelling))
        .collect();
    let read_as = |spelling: &str| {
        items
            .iter()
            .position(|c| c.syllables.get(index).is_some_and(|s| s == spelling))
            .unwrap_or(usize::MAX)
    };
    others.sort_by_cached_key(|s| (Reverse(s.len()), read_as(s), s.clone()));
    choices.extend(others);
    choices
}

fn ranked(index: usize, position: &Position, items: &[Candidate]) -> Vec<String> {
    let rank = |alternative: &str| {
        items
            .iter()
            .position(|c| {
                c.syllables
                    .get(index)
                    .is_some_and(|syllable| syllable.starts_with(alternative))
            })
            .unwrap_or(usize::MAX)
    };
    let mut choices = position.alternatives.clone();
    choices.sort_by_cached_key(|alternative| (rank(alternative), alternative.clone()));
    choices
}

/// 拼音行：每个位置取首选候选读到的那种写法，首选没覆盖到的位置取第一种写法，再接切不动的尾巴。
pub(super) fn marked(shape: &Shape, first: Option<&Candidate>, tail: &str) -> String {
    let mut parts: Vec<&str> = Vec::with_capacity(shape.positions.len() + 1);
    for (index, position) in shape.positions.iter().enumerate() {
        let read = first
            .and_then(|c| c.syllables.get(index))
            .and_then(|syllable| {
                position
                    .alternatives
                    .iter()
                    .find(|a| syllable.starts_with(a.as_str()))
            });
        parts.push(read.unwrap_or(&position.alternatives[0]));
    }
    if !tail.is_empty() {
        parts.push(tail);
    }
    parts.join("'")
}
