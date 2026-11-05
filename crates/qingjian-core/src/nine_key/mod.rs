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

/// 叹词的读法与字。音节表不收这些（`n` / `m` 在全拼与双拼里是简拼的声母），词库里也没有这些读音，
/// 九宫格不这样补就选不到 嗯：拼音列列出 ng / hm / hng，选定后自成一个位置、候选给这个字；
/// `n` / `m` 照旧当声母，只在整段就是这一个字母时多给一个字（见 `Engine::nine_key_interjection`）。
pub const INTERJECTIONS: &[(&str, &str)] = &[
    ("ng", "嗯"),
    ("hng", "哼"),
    ("hm", "噷"),
    ("n", "嗯"),
    ("m", "呣"),
];

/// 叹词读法对应的字。
pub fn interjection(spelling: &str) -> Option<&'static str> {
    INTERJECTIONS
        .iter()
        .find(|(text, _)| *text == spelling)
        .map(|(_, character)| *character)
}

/// 从输入的 `start` 字节起能读成的全部写法：各种长度的完整音节、叹词读法（[`INTERJECTIONS`]）与一码的首字母，长的在前。
/// 拼音列除了最优切法的那个位置，还要能选更短的音节与单个字母，否则别的切法上的字选不到。
pub fn spellings_from(input: &str, start: usize) -> Vec<String> {
    CODE_TABLE.spellings_from(input, start)
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
        // 拼音列选定的音节从开头或 `'` 后起；叹词读法只在这里认，不从 zhang 中间切出 ng
        let pinned_here = start == 0 || bytes[start - 1] == b'\'';
        for end in start + 1..=limit {
            if bytes[end - 1] == b'\'' {
                break;
            }
            let last = end == bytes.len();
            let Some(position) = CODE_TABLE.position(&input[start..end], start, last, pinned_here)
            else {
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
