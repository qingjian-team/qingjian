//! 九宫格给壳显示的两样东西：拼音列与拼音行。

use std::cmp::Reverse;

use crate::candidate::Candidate;
use crate::nine_key::{self, Position, Shape};

/// 拼音列：从第一个未选定位置起枚举全部合法完整音节、叹词读法与兜底单字母，不按最长结构切法硬钉顶部。
///
/// 排序分两档：完整音节与叹词在前，单字母在后；档内优先「在当前候选里出现最靠前的音节」，
/// 其次按词库最高字频与用户词频（`syllable_weight`），最后按长度降序与字典序。
pub(super) fn pinyin_choices(
    keys: &str,
    shape: Option<&Shape>,
    items: &[Candidate],
    syllable_weight: impl Fn(&str) -> (u32, u32),
) -> Vec<String> {
    let Some((index, position)) = shape.and_then(Shape::first_open) else {
        return Vec::new();
    };
    let mut choices = nine_key::spellings_from(keys, position.start);
    for alternative in &position.alternatives {
        if !choices.iter().any(|known| known == alternative) {
            choices.push(alternative.clone());
        }
    }
    let rank = |spelling: &str| {
        let prefix_match =
            spelling.len() == 1 || (position.prefix && spelling.len() == position.len);
        items
            .iter()
            .position(|c| {
                c.syllables.get(index).is_some_and(|syllable| {
                    if prefix_match {
                        syllable.starts_with(spelling)
                    } else {
                        syllable == spelling
                    }
                })
            })
            .unwrap_or(usize::MAX)
    };
    choices.sort_by_cached_key(|spelling| {
        let single = usize::from(spelling.len() == 1);
        let candidate_rank = rank(spelling);
        let (user_weight, dict_freq) = syllable_weight(spelling);
        (
            single,
            candidate_rank,
            Reverse(user_weight),
            Reverse(dict_freq),
            Reverse(spelling.len()),
            spelling.clone(),
        )
    });
    choices
}

/// 拼音行：优先挑能对上首选候选音节的那种切法，避免首选来自非首切法时拼音行与候选脱节；
/// 对不上时回退最优切法。每个位置取首选候选读到的那种写法，没覆盖到的取第一种写法，再接切不动的尾巴。
pub(super) fn marked(shapes: &[Shape], first: Option<&Candidate>, tail: &str) -> Option<String> {
    let shape = first
        .and_then(|candidate| shapes.iter().find(|shape| shape_matches(shape, candidate)))
        .or_else(|| shapes.first())?;
    let mut parts: Vec<&str> = Vec::with_capacity(shape.positions.len() + 1);
    for (index, position) in shape.positions.iter().enumerate() {
        let read = first
            .and_then(|c| c.syllables.get(index))
            .and_then(|syllable| position_match(position, syllable));
        parts.push(read.unwrap_or(&position.alternatives[0]));
    }
    if !tail.is_empty() {
        parts.push(tail);
    }
    Some(parts.join("'"))
}

fn shape_matches(shape: &Shape, candidate: &Candidate) -> bool {
    !candidate.syllables.is_empty()
        && candidate.syllables.len() <= shape.positions.len()
        && shape
            .positions
            .iter()
            .zip(&candidate.syllables)
            .all(|(position, syllable)| position_match(position, syllable).is_some())
}

fn position_match<'a>(position: &'a Position, syllable: &str) -> Option<&'a str> {
    position
        .alternatives
        .iter()
        .find(|a| syllable == a.as_str())
        .or_else(|| {
            position
                .prefix
                .then(|| {
                    position
                        .alternatives
                        .iter()
                        .find(|a| syllable.starts_with(a.as_str()))
                })
                .flatten()
        })
        .map(String::as_str)
}
