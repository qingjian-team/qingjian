//! 九宫格拼音：数字串（可混着已选定的字母）切成若干位置，每个位置带同码的全部写法。
//!
//! 只在边界上做动态规划、不展开读法；写法交给词库的多写法查询与整句词图去分辨。设计见 `docs/design/nine-key.md`。

mod position;
mod shape;
mod table;

pub use position::Position;
pub use shape::Shape;

use crate::parser::MAX_SYLLABLE_LEN;
use table::CODE_TABLE;

/// 每个边界留几种形状、最后交出几种。数字串的边界歧义比全拼多得多（`744` 是 shi 也是 qi + h…），
/// 排序键只看结构，留少了整句要的那种切法会被挤掉。
const BEAM: usize = 12;

/// 字母在九宫格上的数字键；不是小写字母返回 `None`。
pub fn digit_of(letter: char) -> Option<char> {
    Some(match letter {
        'a'..='c' => '2',
        'd'..='f' => '3',
        'g'..='i' => '4',
        'j'..='l' => '5',
        'm'..='o' => '6',
        'p'..='s' => '7',
        't'..='v' => '8',
        'w'..='z' => '9',
        _ => return None,
    })
}

/// 九宫格缓冲区里的合法字符：`2`–`9`、已选定的小写字母、分词的 `'`。
pub fn is_key(c: char) -> bool {
    matches!(c, '2'..='9' | 'a'..='z' | '\'')
}

/// 把输入切成最优的几种形状（最多 `BEAM` 种，最优在前）。切不到末尾时退到能切到的最长前缀，
/// 返回值第二项是切掉的尾巴。第一个字符都切不动返回空。
pub fn segment(input: &str) -> (Vec<Shape>, &str) {
    let bytes = input.as_bytes();
    let mut reached: Vec<Vec<Shape>> = vec![Vec::new(); bytes.len() + 1];
    reached[0].push(Shape::default());
    for start in 0..bytes.len() {
        if reached[start].is_empty() {
            continue;
        }
        if bytes[start] == b'\'' {
            if start > 0 {
                let shapes = reached[start].clone();
                merge(&mut reached[start + 1], shapes);
            }
            continue;
        }
        let limit = (start + MAX_SYLLABLE_LEN).min(bytes.len());
        for end in start + 1..=limit {
            if bytes[end - 1] == b'\'' {
                break;
            }
            let last = end == bytes.len();
            let Some(position) = CODE_TABLE.position(&input[start..end], start, last) else {
                continue;
            };
            let extended: Vec<Shape> = reached[start]
                .iter()
                .map(|shape| shape.extended(position.clone()))
                .collect();
            merge(&mut reached[end], extended);
        }
    }
    let end = (1..=bytes.len())
        .rev()
        .find(|&end| !reached[end].is_empty())
        .unwrap_or(0);
    if end == 0 {
        return (Vec::new(), input);
    }
    (std::mem::take(&mut reached[end]), &input[end..])
}

/// 并进一个边界的形状表，按排序键留前 [`BEAM`] 种。
fn merge(into: &mut Vec<Shape>, shapes: Vec<Shape>) {
    into.extend(shapes);
    into.sort_by_cached_key(Shape::sort_key);
    into.truncate(BEAM);
}

#[cfg(test)]
mod tests;
