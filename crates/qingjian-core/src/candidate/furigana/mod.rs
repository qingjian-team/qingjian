//! 振り仮名：把日文译词的整体假名读音对到各段汉字上（`開発する` + `かいはつする` → 開発(かいはつ)する）。
//!
//! 译词里的假名段在读音里原样出现，拿它们当锚点，锚点之间的假名就是相邻汉字段的读音；
//! 多种对法时取第一种可行的。对不上（读音与写法的假名不一致）就整体注在后面。

mod segment;

pub use segment::FuriganaSegment;

/// 把 `text` 切成汉字段与非汉字段，汉字段带上从 `reading` 里对出来的假名。读音与写法一样（纯假名词）时只有一段、不带读音。
pub fn furigana(text: &str, reading: &str) -> Vec<FuriganaSegment> {
    let runs = split_runs(text);
    let reading_chars: Vec<char> = reading.chars().map(to_hiragana).collect();
    if !runs.iter().any(|(_, kanji)| *kanji) {
        return vec![FuriganaSegment {
            text: text.to_owned(),
            reading: None,
        }];
    }
    let mut readings = Vec::with_capacity(runs.len());
    if align(&runs, &reading_chars, 0, &mut readings) {
        runs.iter()
            .zip(readings)
            .map(|((run, _), reading)| FuriganaSegment {
                text: run.clone(),
                reading,
            })
            .collect()
    } else {
        vec![FuriganaSegment {
            text: text.to_owned(),
            reading: Some(reading.to_owned()),
        }]
    }
}

/// 连续的汉字为一段、连续的非汉字为一段：(段, 是否汉字)。
fn split_runs(text: &str) -> Vec<(String, bool)> {
    let mut runs: Vec<(String, bool)> = Vec::new();
    for c in text.chars() {
        let kanji = is_kanji(c);
        match runs.last_mut() {
            Some((run, last_kanji)) if *last_kanji == kanji => run.push(c),
            _ => runs.push((c.to_string(), kanji)),
        }
    }
    runs
}

/// 回溯对齐：非汉字段必须与读音里对应位置逐字相同（假名先统一成平假名），汉字段取到下一个锚点之间的假名，至少一个。
fn align(
    runs: &[(String, bool)],
    reading: &[char],
    position: usize,
    out: &mut Vec<Option<String>>,
) -> bool {
    let Some(((run, kanji), rest)) = runs.split_first() else {
        return position == reading.len();
    };
    if !*kanji {
        let chars: Vec<char> = run.chars().map(to_hiragana).collect();
        if reading[position..].starts_with(&chars) {
            out.push(None);
            if align(rest, reading, position + chars.len(), out) {
                return true;
            }
            out.pop();
        }
        return false;
    }
    // 最后一段汉字吃掉剩下的全部；中间的汉字段从短到长试，让后面的锚点决定边界
    if rest.is_empty() {
        if position < reading.len() {
            out.push(Some(reading[position..].iter().collect()));
            return true;
        }
        return false;
    }
    for end in position + 1..=reading.len() {
        out.push(Some(reading[position..end].iter().collect()));
        if align(rest, reading, end, out) {
            return true;
        }
        out.pop();
    }
    false
}

/// 汉字（含 々 〆）。
fn is_kanji(c: char) -> bool {
    matches!(c, '\u{4e00}'..='\u{9fff}' | '\u{3400}'..='\u{4dbf}' | '々' | '〆')
}

/// 片假名统一成平假名，长音符等原样。
fn to_hiragana(c: char) -> char {
    match c {
        '\u{30a1}'..='\u{30f6}' => char::from_u32(c as u32 - 0x60).unwrap_or(c),
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(text: &str, reading: &str) -> String {
        furigana(text, reading)
            .iter()
            .map(|s| match &s.reading {
                Some(r) => format!("{}({r})", s.text),
                None => s.text.clone(),
            })
            .collect()
    }

    #[test]
    fn annotates_each_kanji_run() {
        assert_eq!(render("開発する", "かいはつする"), "開発(かいはつ)する");
        assert_eq!(
            render("食事を始める", "しょくじをはじめる"),
            "食事(しょくじ)を始(はじ)める"
        );
        assert_eq!(render("今日", "きょう"), "今日(きょう)");
        assert_eq!(render("私たち", "わたしたち"), "私(わたし)たち");
        assert_eq!(render("お茶", "おちゃ"), "お茶(ちゃ)");
        assert_eq!(render("人々", "ひとびと"), "人々(ひとびと)");
    }

    #[test]
    fn kana_words_and_mismatches() {
        // 纯假名词不注
        assert_eq!(render("こんにちは", "こんにちは"), "こんにちは");
        assert_eq!(render("ウィーチャット", "うぃーちゃっと"), "ウィーチャット");
        // 读音与写法里的假名对不上：整体注在后面
        assert_eq!(render("開発する", "かいはつ"), "開発する(かいはつ)");
        assert_eq!(render("開発する", "かいはつした"), "開発する(かいはつした)");
    }
}
