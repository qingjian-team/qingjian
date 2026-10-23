//! 注音按敲的声调排序：单字声调表 `data/tones.tsv`（`qingjian-dict-convert tones` 从通用词表生成）。
//!
//! 词库不带声调，查词仍按无调拼音；这里给每个候选算一个声调代价，排序把代价小的放前面。
//! 一个字的代价：敲的正是它最常见的读音 0；是它的次要读音（吗 在 吗啡 里读 ma3、抹 平常读 mo3 而 抹布 读 ma1）
//! 或表里查不到 1；对不上 [`MISMATCH`]。次要读音要让一步，因为语言模型的字频不分读音（抹 的字频大半来自 mo3）；
//! 查不到的不算对不上：宁可少排，也不把没收录读音的字压到最后。
use std::collections::HashMap;
use std::sync::LazyLock;

/// 一个字对不上声调的代价，比任何「次要读音 / 查不到」的累计都重（词最多几个字）。
pub const MISMATCH: u8 = 16;

/// 字 → [(无调音节, 声调位集：第 n 位表示 n 声，轻声是 5)]，外加最常见的读音（无调音节, 声调）。
/// 表里一个字的读音按通用词表排序号首次出现的顺序排，第一个就是最常见的。
type Table = HashMap<char, (Vec<(&'static str, u8)>, (&'static str, u8))>;

static TABLE: LazyLock<Table> = LazyLock::new(|| parse(include_str!("../../data/tones.tsv")));

fn parse(source: &'static str) -> Table {
    let mut table = Table::new();
    for line in source.lines() {
        if line.starts_with('#') {
            continue;
        }
        let Some((word, readings)) = line.split_once('\t') else {
            continue;
        };
        let Some(ch) = word.chars().next() else {
            continue;
        };
        let mut syllables: Vec<(&'static str, u8)> = Vec::new();
        let mut first = None;
        for reading in readings.split(' ') {
            let Some(tone) = reading
                .chars()
                .last()
                .and_then(|c| c.to_digit(10))
                .and_then(|d| u8::try_from(d).ok())
            else {
                continue;
            };
            let syllable = &reading[..reading.len() - 1];
            first.get_or_insert((syllable, tone));
            match syllables.iter_mut().find(|(s, _)| *s == syllable) {
                Some((_, mask)) => *mask |= 1 << tone,
                None => syllables.push((syllable, 1 << tone)),
            }
        }
        if let Some(first) = first {
            table.insert(ch, (syllables, first));
        }
    }
    table
}

/// `ch` 读 `syllable` 时的声调：是它最常见的读音就取那个声调，否则这个音只有一种声调才取；拿不准是 0。
pub fn usual(ch: char, syllable: &str) -> u8 {
    let Some((readings, first)) = TABLE.get(&ch) else {
        return 0;
    };
    if first.0 == syllable {
        return first.1;
    }
    match readings.iter().find(|(s, _)| *s == syllable) {
        Some((_, mask)) if mask.count_ones() == 1 => mask.trailing_zeros() as u8,
        _ => 0,
    }
}

/// 候选 `text`（按字对应词库读音 `syllables`）对 `tones`（每个音节敲的声调，None 是没敲）的声调代价，见模块文档。
///
/// 多字词里非首字敲了轻声不计代价：轻声随词变（妈妈 第二个字 表里记一声）。字数与音节数对不上的词不比。
pub fn cost<'a>(text: &str, syllables: impl Iterator<Item = &'a str>, tones: &[Option<u8>]) -> u8 {
    if tones.iter().all(Option::is_none) {
        return 0;
    }
    let syllables: Vec<&str> = syllables.collect();
    if text.chars().count() != syllables.len() {
        return 0;
    }
    let mut total: u8 = 0;
    for (index, ((ch, syllable), tone)) in text.chars().zip(syllables).zip(tones).enumerate() {
        let Some(tone) = *tone else {
            continue;
        };
        if tone == 5 && index > 0 {
            continue;
        }
        let one = match TABLE.get(&ch) {
            Some((_, first)) if *first == (syllable, tone) => 0,
            Some((readings, _)) => match readings.iter().find(|(s, _)| *s == syllable) {
                Some((_, mask)) if mask & (1 << tone) == 0 => MISMATCH,
                _ => 1,
            },
            None => 1,
        };
        total = total.saturating_add(one);
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cost_of(text: &str, pinyin: &str, tones: &[Option<u8>]) -> u8 {
        cost(text, pinyin.split(' '), tones)
    }

    #[test]
    fn single_characters_follow_typed_tone() {
        assert_eq!(cost_of("妈", "ma", &[Some(1)]), 0);
        assert_eq!(cost_of("吗", "ma", &[Some(1)]), MISMATCH);
        assert_eq!(cost_of("马", "ma", &[Some(3)]), 0);
        assert_eq!(cost_of("妈", "ma", &[Some(3)]), MISMATCH);
        assert_eq!(cost_of("吗", "ma", &[Some(5)]), 0);
    }

    #[test]
    fn secondary_readings_yield() {
        // 吗啡 的 ma3、抹布 的 ma1 都是次要读音
        assert_eq!(cost_of("吗", "ma", &[Some(3)]), 1);
        assert_eq!(cost_of("抹", "ma", &[Some(1)]), 1);
        assert_eq!(cost_of("抹", "mo", &[Some(3)]), 0);
    }

    #[test]
    fn untyped_or_unknown_never_mismatches() {
        assert_eq!(cost_of("吗", "ma", &[None]), 0);
        assert_eq!(cost_of("吗", "ma", &[]), 0);
        // 表里没有 么 读 ma 的记录
        assert_eq!(cost_of("么", "ma", &[Some(1)]), 1);
    }

    #[test]
    fn neutral_tone_inside_words_is_free() {
        assert_eq!(cost_of("妈妈", "ma ma", &[Some(1), Some(5)]), 0);
        assert_eq!(cost_of("妈妈", "ma ma", &[Some(1), Some(3)]), MISMATCH);
        // 前缀候选：字比音节少，只比前面的
        assert_eq!(cost_of("妈", "ma", &[Some(1), Some(3)]), 0);
    }

    #[test]
    fn usual_tone_of_a_character() {
        assert_eq!(usual('的', "de"), 5);
        assert_eq!(usual('的', "di"), 0);
        assert_eq!(usual('好', "hao"), 3);
        assert_eq!(usual('好', "xyz"), 0);
    }

    #[test]
    fn parses_first_reading_and_masks() {
        let table = parse("# 注释\n的\tde5 di4 di2\n");
        let (readings, first) = &table[&'的'];
        assert_eq!(*first, ("de", 5));
        assert_eq!(*readings, [("de", 1 << 5), ("di", (1 << 4) | (1 << 2))]);
    }
}
