//! CMU 发音词典（`cmudict.dict`）→ `english-ipa.tsv`：`词\t/IPA/`，随包给英文候选显示音标。
//!
//! CMUdict 的读音是 **ARPAbet**（`K AA1 R G OW2`），带数字调号：`1` 主重音、`2` 次重音、`0` 无重音。
//! 这里换成常见词典里那种 General American 音标（`/ˈkɑrɡoʊ/`）。CMUdict 本身就是美式发音库，
//! 没有英式；想要英式得另找数据源。
//!
//! 规则与取舍：
//! - **一个词只留一条**：CMUdict 给多音词按 `word(2)`、`word(3)` 列出变体，只取不带序号的第一个
//!   （最常用的那个）。多音与地区差异留给以后。
//! - **重音挪到音节首**：ARPAbet 把调号标在元音上，IPA 要写在音节最前面——
//!   `cargo` 是 `/ˈkɑrɡoʊ/`，不是 `/kˈɑrɡoʊ/`。做法是找到带调号的元音，从它往前推到音节的起点
//!   （即上一个元音之后、或序列开头），把标记插在那里。
//! - **只收纯小写字母的词**：带撇号（`'bout`）、连字符、数字的条目跳过——那些在候选里基本不出现，
//!   留着只会让表变大、查询时还得多做大小写处理。查询时统一转小写。
//! - 没命中就是没这条数据，调用方 `reading = None`，候选照常显示、只是不带音标。
//!
//! **已知的覆盖缺口**：CMUdict 只收「词形变化有音变」的那部分变体，规则派生出来的复数 / 过去式 /
//! 进行式（`abacuses`、`abscessed`、`abstracting`）大多不在里面。与产品 `english.tsv` 对了一遍，
//! 9.4 万词里约 **57%** 有音标，剩下的是这些派生词形。要补上得按后缀反推词根
//! （`-s` / `-ed` / `-ing` / `-er` / `-est` 各有读音规则），属于后续阶段的事；
//! 这里不猜——查不到就不显示，比显示错的好。

use std::collections::HashSet;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::error::ConvertError;

/// ARPAbet → IPA（General American）。重音调号单独处理，不在这张表里。
const PHONEMES: &[(&str, &str)] = &[
    // 元音
    ("AA", "ɑ"),
    ("AE", "æ"),
    ("AH", "ʌ"),
    ("AO", "ɔ"),
    ("AW", "aʊ"),
    ("AY", "aɪ"),
    ("EH", "ɛ"),
    ("ER", "ɝ"),
    ("EY", "eɪ"),
    ("IH", "ɪ"),
    ("IY", "i"),
    ("OW", "oʊ"),
    ("OY", "ɔɪ"),
    ("UH", "ʊ"),
    ("UW", "u"),
    // 辅音
    ("B", "b"),
    ("CH", "tʃ"),
    ("D", "d"),
    ("DH", "ð"),
    ("F", "f"),
    ("G", "ɡ"),
    ("HH", "h"),
    ("JH", "dʒ"),
    ("K", "k"),
    ("L", "l"),
    ("M", "m"),
    ("N", "n"),
    ("NG", "ŋ"),
    ("P", "p"),
    ("R", "r"),
    ("S", "s"),
    ("SH", "ʃ"),
    ("T", "t"),
    ("TH", "θ"),
    ("V", "v"),
    ("W", "w"),
    ("Y", "j"),
    ("Z", "z"),
    ("ZH", "ʒ"),
];

/// 元音：一个单词有几个元音就有几个音节。
const VOWELS: &[&str] = &[
    "AA", "AE", "AH", "AO", "AW", "AY", "EH", "ER", "EY", "IH", "IY", "OW", "OY", "UH", "UW",
];

/// 重音调号；无重音的 `AH` 要按中央元音处理，见 [`to_ipa`]。
fn split_stress(raw: &str) -> (&str, Option<char>) {
    if let Some(base) = raw.strip_suffix('1') {
        return (base, Some('ˈ'));
    }
    if let Some(base) = raw.strip_suffix('2') {
        return (base, Some('ˌ'));
    }
    (raw.strip_suffix('0').unwrap_or(raw), None)
}

/// 把一条 ARPAbet 读音转成 IPA；任何音素认不出就返回 `None`（宁可没有，也不要画出乱码）。
///
/// 三处不能机械查表的地方：
/// - **重音挪到音节首**：ARPAbet 把调号标在元音上，IPA 要写在音节最前面——
///   `cargo` 是 `/ˈkɑrɡoʊ/` 不是 `/kˈɑrɡoʊ/`。做法是先划音节，再把标记插到该音节第一个音素前。
/// - **音节边界按最大起首音**：两个元音之间的辅音，**全部**归后一音节当声母，除非它一个都不剩
///   （前一音节必须留住自己的收尾）。`computer` 是 `kə|mpju` → `/kəˈmpjutɝ/`（`m` 是声母），
///   `cargo` 是 `kɑr|ɡoʊ`（`r` 是上一音节的儿化收尾，`ɡ` 是声母）——两种都对，
///   区别在于 `r` 在元音后要跟前一个元音走（英语的 r 属于前一音节）。
/// - **无重音的 `AH` 是 `ə` 不是 `ʌ`**：CMUdict 用同一个 `AH` 表示两个不同的音，靠调号区分——
///   带调号的 `AH1`/`AH2` 是 STRUT 元音 `ʌ`，不带调号的 `AH0` 是 schwa `ə`
///   （`hello` 的 `HH AH0 L` → `həˈloʊ`，不区分就成了 `/hʌˈloʊ/`）。
fn to_ipa(phonemes: &[&str]) -> Option<String> {
    let parts: Vec<(&str, Option<char>)> = phonemes.iter().map(|raw| split_stress(raw)).collect();

    // 划音节：每个音节 = [起首辅音…] + 元音 + [其余]。
    // 边界规则：两个元音之间的辅音全部归后一音节当声母，只有元音后的 R 留在前一音节
    // （英语的 r 属于前一音节：`cargo` 是 `kɑr|ɡoʊ`）。
    let vowel_positions: Vec<usize> = parts
        .iter()
        .enumerate()
        .filter(|(_, (base, _))| VOWELS.contains(base))
        .map(|(index, _)| index)
        .collect();
    if vowel_positions.is_empty() {
        return None;
    }
    let mut boundaries: Vec<usize> = vec![0];
    for pair in vowel_positions.windows(2) {
        let (previous, next) = (pair[0], pair[1]);
        let mut boundary = previous + 1;
        while boundary < next && parts[boundary].0 == "R" {
            boundary += 1;
        }
        // 若整段都是 R，边界不越过 next，避免把元音划给前一音节
        boundaries.push(boundary.min(next));
    }

    // 每个元音所属音节的起点：边界列表里不超过它的最后一个
    let mut stress_at: Vec<Option<char>> = vec![None; parts.len()];
    for (index, (base, stress)) in parts.iter().enumerate() {
        let Some(mark) = stress else { continue };
        if !VOWELS.contains(base) {
            // 调号只该出现在元音上；出现在别处说明数据是坏的，别猜
            return None;
        }
        let start = boundaries
            .iter()
            .rev()
            .find(|&&boundary| boundary <= index)
            .copied()
            .unwrap_or(0);
        stress_at[start] = Some(*mark);
    }

    let mut out = String::new();
    for (index, (base, stress)) in parts.iter().enumerate() {
        if let Some(mark) = stress_at[index] {
            out.push(mark);
        }
        let ipa = match (*base, stress.is_some()) {
            // 无重音的 AH 是 schwa
            ("AH", false) => "ə",
            _ => PHONEMES
                .iter()
                .find(|(arpabet, _)| arpabet == base)
                .map(|(_, ipa)| *ipa)?,
        };
        out.push_str(ipa);
    }
    Some(out)
}

/// 只收这种词：全是 ASCII 小写字母，长度 1..=32（再长的不像英文词）。
fn acceptable(word: &str) -> bool {
    !word.is_empty() && word.len() <= 32 && word.bytes().all(|b| b.is_ascii_lowercase())
}

pub fn convert(input: &Path, output: &Path) -> Result<(), ConvertError> {
    let source = std::fs::read_to_string(input)?;
    let mut rows: Vec<(String, String)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut skipped = 0usize;
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(";;;") {
            continue;
        }
        let mut fields = line.split_whitespace();
        let Some(word) = fields.next() else { continue };
        let word = word.to_ascii_lowercase();
        // `word(2)` 这类变体不取：第一个就是最常用的
        if word.bytes().any(|b| b.is_ascii_digit()) || !acceptable(&word) {
            skipped += 1;
            continue;
        }
        if !seen.insert(word.clone()) {
            continue;
        }
        let phonemes: Vec<&str> = fields.collect();
        let Some(ipa) = to_ipa(&phonemes) else {
            skipped += 1;
            continue;
        };
        rows.push((word, format!("/{ipa}/")));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));

    let mut file = BufWriter::new(std::fs::File::create(output)?);
    writeln!(
        file,
        "# 由 qingjian-dict-convert ipa 生成，来源 CMU Pronouncing Dictionary（BSD-2-Clause）。词\\t/音标/（General American）"
    )?;
    for (word, ipa) in &rows {
        writeln!(file, "{word}\t{ipa}")?;
    }
    file.flush()?;
    tracing::info!(
        path = %output.display(),
        entries = rows.len(),
        skipped,
        "音标表写出完成"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stress_goes_to_the_start_of_its_syllable() {
        // cargo  K AA1 R G OW2 → 主重音在第一音节（ˈkɑr），次重音在第二音节（ˌɡoʊ）
        assert_eq!(
            to_ipa(&["K", "AA1", "R", "G", "OW2"]),
            Some("ˈkɑrˌɡoʊ".to_owned())
        );
        // hello  HH AH0 L OW1 → 主重音在第二音节，l 是它的声母
        assert_eq!(
            to_ipa(&["HH", "AH0", "L", "OW1"]),
            Some("həˈloʊ".to_owned())
        );
        // computer  K AH0 M P Y UW1 T ER0 → m 是第二音节的声母，跟着 pyu 走
        assert_eq!(
            to_ipa(&["K", "AH0", "M", "P", "Y", "UW1", "T", "ER0"]),
            Some("kəˈmpjutɝ".to_owned())
        );
    }

    /// CMUdict 用同一个 `AH` 表示两个音，靠调号区分：带调号是 `ʌ`，不带是 schwa `ə`。
    #[test]
    fn unstressed_ah_is_schwa() {
        assert_eq!(to_ipa(&["AH1"]), Some("ˈʌ".to_owned()));
        assert_eq!(to_ipa(&["AH0"]), Some("ə".to_owned()));
    }

    #[test]
    fn unknown_phoneme_yields_nothing() {
        assert_eq!(to_ipa(&["K", "XY1"]), None);
    }

    #[test]
    fn only_plain_lowercase_words_pass_the_filter() {
        assert!(acceptable("cargo"));
        assert!(!acceptable("'bout"));
        assert!(!acceptable("e-mail"));
        assert!(!acceptable("Cargo"));
        assert!(!acceptable(""));
    }
}
