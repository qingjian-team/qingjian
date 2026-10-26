//! 九宫格给壳显示的两样东西：拼音列与拼音行。

use crate::candidate::Candidate;
use crate::nine_key::Shape;

/// 拼音列：第一个没定下来的位置上的全部写法，按「带这个音节的最靠前候选」排序，没有候选的排最后。
pub(super) fn pinyin_choices(shape: Option<&Shape>, items: &[Candidate]) -> Vec<String> {
    let Some((index, position)) = shape.and_then(Shape::first_open) else {
        return Vec::new();
    };
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
