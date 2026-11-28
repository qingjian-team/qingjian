//! 九宫格给壳显示的两样东西：拼音列与拼音行。

use std::cmp::Reverse;

use crate::candidate::Candidate;
use crate::nine_key::{self, Lattice, Position, Shape};

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

/// 拼音行：沿音节 DAG 把首选候选的音节逐段对齐回输入，保证任何 DAG 边界切法胜出时拼音行都与首选一致；
/// 候选只覆盖前面一段时剩余部分按结构最优切法补齐；对不上时回退最优结构切法。最后接切不动的尾巴。
pub(super) fn marked(
    lattice: &Lattice,
    fallback: Option<&Shape>,
    first: Option<&Candidate>,
    tail: &str,
) -> Option<String> {
    if lattice.is_empty() {
        return None;
    }
    let mut parts: Vec<&str> = Vec::new();
    if let Some(candidate) = first.filter(|c| !c.syllables.is_empty())
        && let Some(end_offset) = align_candidate(lattice, 0, &candidate.syllables, &mut parts)
    {
        for position in lattice.suffix_positions(end_offset) {
            parts.push(&position.alternatives[0]);
        }
    } else {
        let shape = fallback?;
        for position in &shape.positions {
            parts.push(&position.alternatives[0]);
        }
    }
    if !tail.is_empty() {
        parts.push(tail);
    }
    Some(parts.join("'"))
}

/// 在 DAG 上从字节偏移 `offset` 起把 `syllables` 逐个匹配成边写法，成功返回落到的字节偏移。
fn align_candidate<'a>(
    lattice: &'a Lattice,
    offset: usize,
    syllables: &[String],
    parts: &mut Vec<&'a str>,
) -> Option<usize> {
    let Some((syllable, rest)) = syllables.split_first() else {
        return Some(offset);
    };
    // 优先完整等长匹配，再退回前缀匹配（简拼或末尾未打完音节）
    for exact_pass in [true, false] {
        for edge in lattice.edges_at(offset) {
            let Some(written) = position_match(&edge.position, syllable) else {
                continue;
            };
            let is_exact = written.len() == syllable.len();
            if is_exact != exact_pass {
                continue;
            }
            parts.push(written);
            if let Some(end) = align_candidate(lattice, edge.next, rest, parts) {
                return Some(end);
            }
            parts.pop();
        }
    }
    None
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
