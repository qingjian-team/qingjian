//! 双拼与全拼混输：同一个输入里两种读法都试，给引擎若干条解码结果。
//!
//! 纯双拼的读法原样保留（[`Layout::decode_fuzzy`]），再加一条整段按全拼切的读法，以及两种读法
//! 在同一输入的音节边界上交替的读法（`wodemkzijiaozhangsan`：前半双拼、后半全拼）。每条读法都是
//! 一个 [`Decoded`]，候选与整句在这些读法之间按分数比，不固定哪条读法在前。
//!
//! 每个位置最多留 [`MAX_PATHS`] 条部分读法，控制歧义搜索量；纯双拼与整段全拼两条始终保留。

use qingjian_dictionary::canonical_syllable;

use super::{Decoded, Layout, Unit};
use crate::fuzzy::FuzzyRules;
use crate::parser;

/// 每个位置最多保留几条部分读法。
const MAX_PATHS: usize = 8;

impl Layout {
    /// 混输解码：纯双拼、整段全拼、以及两者在音节边界上交替的读法，按这个顺序返回（都解不动的尾巴留在各自结果里）。
    /// 纯双拼那条与 `decode_fuzzy` 完全相同，混输关掉时行为不变。
    pub fn decode_mixed(&self, keys: &str, fuzzy: FuzzyRules) -> Vec<Decoded> {
        let mut readings = vec![self.decode_fuzzy(keys, fuzzy)];
        if let Some(full) = full_pinyin(keys) {
            push_reading(&mut readings, full);
        }
        for hybrid in self.hybrids(keys, fuzzy) {
            push_reading(&mut readings, hybrid);
        }
        readings
    }

    /// 在音节边界上交替的读法：每一位都可以按双拼两键切，也可以按全拼一到六个字母切。
    fn hybrids(&self, keys: &str, fuzzy: FuzzyRules) -> Vec<Decoded> {
        let chars: Vec<char> = keys.chars().collect();
        let mut states: Vec<Vec<Vec<Unit>>> = vec![Vec::new(); chars.len() + 1];
        states[0].push(Vec::new());
        for index in 0..chars.len() {
            let paths = std::mem::take(&mut states[index]);
            for path in paths {
                let first = chars[index];
                if first == '\'' {
                    let mut next = path.clone();
                    next.push(Unit::separator());
                    if states[index + 1].len() < MAX_PATHS {
                        states[index + 1].push(next);
                    }
                    continue;
                }
                // 双拼：两键一个完整音节
                if let Some(second) = chars.get(index + 1).copied().filter(|c| *c != '\'')
                    && let Some(pinyin) = self.pair(first, second, fuzzy)
                {
                    let mut next = path.clone();
                    next.push(Unit {
                        keys: [first, second].iter().collect(),
                        pinyin,
                        complete: true,
                    });
                    push_state(&mut states[index + 2], next);
                }
                // 全拼：一到六个字母一个音节（最后一个位置允许没打完的前缀）
                let mut end = index;
                while end < chars.len()
                    && chars[end] != '\''
                    && end - index < parser::MAX_SYLLABLE_LEN
                {
                    end += 1;
                }
                for len in 1..=(end - index) {
                    let text: String = chars[index..index + len].iter().collect();
                    if !text.chars().all(|ch| ch.is_ascii_lowercase()) {
                        break;
                    }
                    let complete = parser::is_syllable(&text);
                    let partial = !complete
                        && index + len == chars.len()
                        && parser::is_syllable_prefix(&text);
                    if !complete && !partial {
                        continue;
                    }
                    let mut next = path.clone();
                    next.push(Unit {
                        keys: text.clone(),
                        pinyin: canonical_syllable(&text).to_owned(),
                        complete,
                    });
                    push_state(&mut states[index + len], next);
                }
            }
        }
        states
            .pop()
            .expect("paths are keyed by position")
            .into_iter()
            .map(|units| Decoded::new(units, String::new()))
            .collect()
    }
}

/// 整段按全拼切的读法：`parser::segment` 里第一条每个音节都打完（或只有末尾没打完）的切分。
/// 不含只靠简拼凑出来的切分：双拼键串常能被读成一串声母（`nihc` → `ni'h'c`），那种读法会
/// 把双拼本身读出的词挤下去（#374）。
fn full_pinyin(keys: &str) -> Option<Decoded> {
    let segmentations = parser::segment(keys).ok()?;
    let segmentation = segmentations.into_iter().find(|segmentation| {
        segmentation
            .syllables
            .iter()
            .enumerate()
            .all(|(index, syllable)| syllable.complete || index + 1 == segmentation.syllables.len())
    })?;
    let mut units = Vec::with_capacity(segmentation.syllables.len() * 2);
    let mut chars = keys.chars().peekable();
    for syllable in &segmentation.syllables {
        while chars.peek() == Some(&'\'') {
            chars.next();
            units.push(Unit::separator());
        }
        for _ in 0..syllable.text.chars().count() {
            chars.next()?;
        }
        units.push(Unit {
            keys: syllable.text.clone(),
            pinyin: syllable.pattern().text.to_owned(),
            complete: syllable.complete,
        });
    }
    // 末尾的分隔符也照原样留着，否则全拼读法比输入少一个键，主切分会被双拼读法抢走
    while chars.peek() == Some(&'\'') {
        chars.next();
        units.push(Unit::separator());
    }
    Some(Decoded::new(units, String::new()))
}

/// 同一个位置再多一条部分读法（同一全拼串不重复，最多 [`MAX_PATHS`] 条）。
fn push_state(state: &mut Vec<Vec<Unit>>, path: Vec<Unit>) {
    if state.len() >= MAX_PATHS || state.iter().any(|existing| same_pinyin(existing, &path)) {
        return;
    }
    state.push(path);
}

/// 两条部分读法解出来的全拼是否相同。
fn same_pinyin(a: &[Unit], b: &[Unit]) -> bool {
    let mut left = a.iter().filter(|u| !u.is_separator());
    let mut right = b.iter().filter(|u| !u.is_separator());
    loop {
        match (left.next(), right.next()) {
            (None, None) => return true,
            (Some(x), Some(y)) if x.pinyin == y.pinyin => {}
            _ => return false,
        }
    }
}

/// 加一条完整读法；全拼串已经出现过就不重复加。
fn push_reading(readings: &mut Vec<Decoded>, reading: Decoded) {
    if readings
        .iter()
        .any(|existing| existing.pinyin() == reading.pinyin())
    {
        return;
    }
    readings.push(reading);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shuangpin::Scheme;

    fn pinyins(readings: &[Decoded]) -> Vec<String> {
        readings
            .iter()
            .map(|reading| reading.pinyin().to_owned())
            .collect()
    }

    #[test]
    fn keeps_the_pure_shuangpin_reading_first() {
        let readings = Scheme::Xiaohe.decode_mixed("nihc", FuzzyRules::default());
        assert_eq!(pinyins(&readings)[0], "ni'hao");
        assert!(readings[0].is_complete());
    }

    #[test]
    fn does_not_read_double_pinyin_keys_as_abbreviated_full_pinyin() {
        // `nihc` 按全拼只能读成 ni'h'c 这种一串声母（简拼），不能算第二种读法，
        // 否则 你还吃 会把 你好 挤下去；`dajx` 同理（打鸡血 vs 大家）。
        for keys in ["nihc", "dajx", "womf"] {
            let readings = Scheme::Xiaohe.decode_mixed(keys, FuzzyRules::default());
            assert_eq!(
                pinyins(&readings).len(),
                1,
                "{keys}: {:?}",
                pinyins(&readings)
            );
        }
    }

    #[test]
    fn adds_the_full_pinyin_reading() {
        let readings = Scheme::Xiaohe.decode_mixed("nihao", FuzzyRules::default());
        assert!(pinyins(&readings).contains(&"ni'hao".to_owned()));
    }

    #[test]
    fn mixes_readings_inside_one_input() {
        // 前半双拼（wo、de 正好也像全拼）、后半只可能是全拼（ming zi）
        let readings = Scheme::Xiaohe.decode_mixed("wodemingzi", FuzzyRules::default());
        assert!(pinyins(&readings).contains(&"wo'de'ming'zi".to_owned()));
    }

    #[test]
    fn readings_are_unique_and_bounded() {
        let readings =
            Scheme::Xiaohe.decode_mixed("zhonghuarenmingongheguo", FuzzyRules::default());
        let mut unique = pinyins(&readings);
        let count = unique.len();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), count);
        assert!(count <= MAX_PATHS * 2);
    }
}
