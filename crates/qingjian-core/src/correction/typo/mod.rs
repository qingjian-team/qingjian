//! 音节级的敲错变体：一个完整音节一处编辑（相邻两键换位、敲到相邻键、多键、少键）之后还是合法音节的那些写法。
//!
//! 词图里每个完整音节除了敲的原样，还按这些变体查词，命中的词扣掉相应代价（见 `TypoCosts`），
//! 让「每个音节都合法、整句却不通」的输入（`meiganxi`）也能出 没关系。整段一处编辑的纠错（`variants`）管切不干净的输入，两者互补。
//!
//! 表按音节表一次算好：每个音节的变体几十个，全部音节几千条。

mod kind;

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::parser::{self, SYLLABLES};
pub use kind::TypoKind;

/// QWERTY 三排字母，算相邻键用。
const ROWS: [&str; 3] = ["qwertyuiop", "asdfghjkl", "zxcvbnm"];

/// 全部音节的敲错变体：音节 → (变体, 类别)。同一变体只留代价最低的类别。
static TABLE: LazyLock<HashMap<&'static str, Vec<(String, TypoKind)>>> = LazyLock::new(|| {
    SYLLABLES
        .iter()
        .map(|syllable| (*syllable, compute_variants(syllable)))
        .collect()
});

/// `syllable` 的敲错变体；不是完整音节（简拼、没打完的前缀）或单字母音节没有变体。
pub fn variants(syllable: &str) -> &'static [(String, TypoKind)] {
    TABLE.get(syllable).map_or(&[], Vec::as_slice)
}

/// `intended` 是不是 `typed` 的一处敲错变体（读取代价用 [`kind`]）。
pub fn is_variant(typed: &str, intended: &str) -> bool {
    kind(typed, intended).is_some()
}

/// 把 `typed` 敲成 `intended` 属于哪类敲错；不是一处敲错返回 `None`。
pub fn kind(typed: &str, intended: &str) -> Option<TypoKind> {
    variants(typed)
        .iter()
        .find(|(text, _)| text == intended)
        .map(|(_, kind)| *kind)
}

/// 两个字母在键盘上是否相邻（同排隔壁，或上下排错位挨着的两个）。
pub fn adjacent(a: char, b: char) -> bool {
    let Some((row_a, col_a)) = position(a) else {
        return false;
    };
    let Some((row_b, col_b)) = position(b) else {
        return false;
    };
    if row_a == row_b {
        return col_a.abs_diff(col_b) == 1;
    }
    if row_a.abs_diff(row_b) != 1 {
        return false;
    }
    // 下一排整体向右错开半个键：上排第 i 个键的下面是下排第 i−1 与第 i 个
    let (upper, lower) = if row_a < row_b {
        (col_a, col_b)
    } else {
        (col_b, col_a)
    };
    lower == upper || lower + 1 == upper
}

fn position(letter: char) -> Option<(usize, usize)> {
    ROWS.iter()
        .enumerate()
        .find_map(|(row, keys)| keys.find(letter).map(|col| (row, col)))
}

/// 一个音节的全部一处编辑（换键只算相邻键）里仍是合法音节的那些。单字母音节（`a` / `e` / `o`）不算：一键之差就是另一个字，谈不上敲错。
fn compute_variants(syllable: &str) -> Vec<(String, TypoKind)> {
    let bytes = syllable.as_bytes();
    let n = bytes.len();
    let mut found: Vec<(String, TypoKind)> = Vec::new();
    if n < 2 {
        return found;
    }
    let mut push = |text: String, kind: TypoKind| {
        if text == syllable || !parser::is_syllable(&text) {
            return;
        }
        match found.iter_mut().find(|(t, _)| *t == text) {
            Some(existing) => {
                if kind.cost() < existing.1.cost() {
                    existing.1 = kind;
                }
            }
            None => found.push((text, kind)),
        }
    };
    for i in 0..n - 1 {
        if bytes[i] != bytes[i + 1] {
            let mut v = bytes.to_vec();
            v.swap(i, i + 1);
            push(String::from_utf8(v).expect("ascii"), TypoKind::Transpose);
        }
    }
    for i in 0..n {
        for letter in b'a'..=b'z' {
            if letter == bytes[i] || !adjacent(bytes[i] as char, letter as char) {
                continue;
            }
            let mut v = bytes.to_vec();
            v[i] = letter;
            push(String::from_utf8(v).expect("ascii"), TypoKind::Substitute);
        }
    }
    for i in 0..n {
        let mut v = bytes.to_vec();
        v.remove(i);
        push(String::from_utf8(v).expect("ascii"), TypoKind::Extra);
    }
    for i in 0..=n {
        for letter in b'a'..=b'z' {
            let mut v = bytes.to_vec();
            v.insert(i, letter);
            push(String::from_utf8(v).expect("ascii"), TypoKind::Missing);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_adjacency() {
        assert!(adjacent('n', 'm'));
        assert!(adjacent('i', 'o'));
        assert!(adjacent('s', 'w'));
        assert!(adjacent('s', 'e'));
        assert!(adjacent('g', 'b'));
        assert!(!adjacent('a', 'k'));
        assert!(!adjacent('q', 'z'));
        assert!(!adjacent('a', 'a'));
    }

    #[test]
    fn variants_are_legal_syllables_one_edit_away() {
        let gan: Vec<&str> = variants("gan").iter().map(|(t, _)| t.as_str()).collect();
        assert!(gan.contains(&"guan"));
        assert!(gan.contains(&"gang"));
        assert!(gan.contains(&"an"));
        assert!(gan.contains(&"han")); // g 与 h 相邻
        assert!(!gan.contains(&"dan")); // g 与 d 不相邻
        assert!(!gan.contains(&"gan"));
        assert!(gan.iter().all(|t| parser::is_syllable(t)));
        assert_eq!(kind("gan", "guan"), Some(TypoKind::Missing));
        assert_eq!(kind("gan", "gang"), Some(TypoKind::Missing));
        assert_eq!(kind("gang", "gan"), Some(TypoKind::Extra));
        assert_eq!(kind("shou", "shuo"), Some(TypoKind::Transpose));
        assert_eq!(kind("ni", "mi"), Some(TypoKind::Substitute));
        assert_eq!(kind("ta", "da"), None);
        assert_eq!(kind("gan", "xi"), None);
        // 单字母音节、简拼、前缀没有变体
        assert!(variants("a").is_empty());
        assert!(variants("zh").is_empty());
        assert!(variants("xia").len() >= 3); // xian / xiao / xi
        assert!(!is_variant("k", "ka"));
    }

    /// 同一变体能由多种编辑得到时留代价最低的：`shuo` → `shou` 既是换位也是两处换键，算换位。
    #[test]
    fn keeps_the_cheapest_kind_per_variant() {
        assert_eq!(kind("shuo", "shou"), Some(TypoKind::Transpose));
    }
}
