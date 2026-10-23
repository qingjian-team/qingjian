//! 通用大词库（青简词库清洗交付的数字声调版：`词\t拼音\t权重`，2–30 字，没有单字）并进基础词库。
//!
//! 权重与我们的语料词频不是同一个尺度：用两边读音相同的词在对数坐标上拟一条直线换算过来。
//! 已清洗的那批（`--cleaned` 给的子集）与基础词库同一档，和 `dict.tsv` 一起打成 `dict.qj`；其余还没清洗的进补充词库 `dicts/general.qj`，
//! 等清洗完一批就挪进前一份。
//! 这些词**都不进语言模型的分词词表**（写在单独的 `dict-cleaned.tsv` / `dict-general.tsv`，`bigram` 只读 `dict.tsv` 与 `dicts/*.tsv`）：
//! 一百多万个低频词当 token 统计，一元最大概率切分会把「们最想」这种片段整块切走，成分词的二元证据就没了（短语层踩过同样的坑）。
//! 整句里它们按词库词频兜底打分（`qingjian_core::sentence::fallback_log_prob`）。

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use qingjian_core::parser::is_syllable;
use qingjian_dictionary::{canonical_syllable, split_tone};

use super::DISPUTED_READING_DIVISOR;
use super::entry::LexiconEntry;
use crate::error::ConvertError;

/// 拟合换算至少要这么多对两边都有的词，少了说明拿错了文件。
const MIN_CALIBRATION_PAIRS: usize = 1000;

/// 一行。
pub struct GeneralRow {
    pub text: String,

    /// 不带声调，ü 写 v。
    pub syllables: Vec<String>,

    /// 与 `syllables` 一一对应，0 是没标。
    pub tones: Vec<u8>,

    pub weight: u64,
}

/// 读整个文件；音节数与字数对不上、有不是拼音音节的行跳过，返回读到的行与跳过的行数。
pub fn load(path: &Path) -> Result<(Vec<GeneralRow>, usize), ConvertError> {
    let source = std::fs::read_to_string(path)?;
    let mut rows = Vec::new();
    let mut skipped = 0usize;
    for line in source.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(text), Some(pinyin), Some(weight)) =
            (fields.next(), fields.next(), fields.next())
        else {
            skipped += 1;
            continue;
        };
        let Ok(weight) = weight.trim().parse::<u64>() else {
            skipped += 1;
            continue;
        };
        let (syllables, tones): (Vec<String>, Vec<u8>) = pinyin
            .split_whitespace()
            .map(|s| {
                let (bare, tone) = split_tone(s);
                (canonical_syllable(bare).to_owned(), tone)
            })
            .unzip();
        if syllables.len() != text.chars().count() || !syllables.iter().all(|s| is_syllable(s)) {
            skipped += 1;
            continue;
        }
        rows.push(GeneralRow {
            text: text.to_owned(),
            syllables,
            tones,
            weight,
        });
    }
    Ok((rows, skipped))
}

/// 权重 → 语料词频：`ln(词频 + 1) = intercept + slope · ln(权重 + 1)`。
#[derive(Debug, Clone, Copy)]
pub struct Scale {
    pub slope: f64,
    pub intercept: f64,
}

impl Scale {
    /// 最小二乘拟合，`pairs` 是（权重，词频）。
    pub fn fit(pairs: &[(u64, u32)]) -> Result<Self, ConvertError> {
        if pairs.len() < MIN_CALIBRATION_PAIRS {
            return Err(ConvertError::Format {
                path: "general".into(),
                line: 0,
                reason: format!(
                    "only {} words shared with the base lexicon, need {MIN_CALIBRATION_PAIRS} to calibrate weights",
                    pairs.len()
                ),
            });
        }
        let n = pairs.len() as f64;
        let points = pairs
            .iter()
            .map(|&(w, f)| ((w as f64 + 1.0).ln(), (f64::from(f) + 1.0).ln()));
        let (sx, sy) = points
            .clone()
            .fold((0.0, 0.0), |(a, b), (x, y)| (a + x, b + y));
        let (mx, my) = (sx / n, sy / n);
        let (cov, var) = points.fold((0.0, 0.0), |(c, v), (x, y)| {
            (c + (x - mx) * (y - my), v + (x - mx) * (x - mx))
        });
        let slope = cov / var;
        Ok(Self {
            slope,
            intercept: my - slope * mx,
        })
    }

    pub fn apply(&self, weight: u64) -> u32 {
        let log = self.intercept + self.slope * (weight as f64 + 1.0).ln();
        (log.exp() - 1.0).round().clamp(1.0, f64::from(u32::MAX)) as u32
    }
}

/// 词面 + 音节 → 词条，与 `lexicon` 里的表同形。
pub type Entries = BTreeMap<(String, Vec<String>), LexiconEntry>;

/// 并入后要另写的两批：已清洗的（与基础词库同一档）与还没清洗的（补充词库）。
pub struct Merged {
    pub cleaned: Entries,
    pub general: Entries,
}

/// 并入通用大词库，`cleaned` 里的词进 [`Merged::cleaned`]，其余进 [`Merged::general`]：
/// - 基础词库已有同一读音的：补上声调，词频不动（与语言模型同一份统计）；
/// - 领域词库不动（缺省关着，挪进基础词库会让 智永 这种人名压过 只用），里面的词与别的新词一样进补充词库；
/// - 词在基础词库、读音不同的：两边都可能错（传说中的 di / 什刹海 shen），新读音降到换算词频的几分之一保留；
/// - 都没有的：按 [`Scale`] 换算词频。
pub fn merge(
    path: &Path,
    cleaned: &HashSet<String>,
    entries: &mut Entries,
) -> Result<Merged, ConvertError> {
    let (rows, skipped) = load(path)?;

    let pairs: Vec<(u64, u32)> = rows
        .iter()
        .filter_map(|row| {
            entries
                .get(&(row.text.clone(), row.syllables.clone()))
                .filter(|e| e.frequency > 0)
                .map(|e| (row.weight, e.frequency))
        })
        .collect();
    let scale = Scale::fit(&pairs)?;
    let base_texts: HashSet<String> = entries.keys().map(|(text, _)| text.clone()).collect();
    let mut merged = Merged {
        cleaned: Entries::new(),
        general: Entries::new(),
    };
    let (mut toned, mut disputed) = (0usize, 0usize);
    for row in rows {
        let key = (row.text, row.syllables);
        if let Some(entry) = entries.get_mut(&key) {
            if entry.tones.is_empty() {
                entry.tones = row.tones;
            }
            toned += 1;
            continue;
        }
        let mut frequency = scale.apply(row.weight);
        if base_texts.contains(&key.0) {
            frequency = (frequency / DISPUTED_READING_DIVISOR).max(1);
            disputed += 1;
        }
        let target = if cleaned.contains(&key.0) {
            &mut merged.cleaned
        } else {
            &mut merged.general
        };
        target.insert(
            key.clone(),
            LexiconEntry {
                text: key.0,
                syllables: key.1,
                frequency,
                tones: row.tones,
            },
        );
    }
    tracing::info!(
        path = %path.display(),
        calibration_pairs = pairs.len(),
        slope = scale.slope,
        intercept = scale.intercept,
        toned,
        disputed_readings = disputed,
        cleaned = merged.cleaned.len(),
        general = merged.general.len(),
        skipped,
        "通用大词库已并入"
    );
    Ok(merged)
}

/// 已清洗子集的词面（与通用大词库同格式的文件，只看第一列）。
pub fn cleaned_texts(path: &Path) -> Result<HashSet<String>, ConvertError> {
    Ok(std::fs::read_to_string(path)?
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split('\t').next().map(str::to_owned))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_tones_and_skips_broken_rows() {
        let path =
            std::env::temp_dir().join(format!("qingjian-general-load-{}.tsv", std::process::id()));
        std::fs::write(
            &path,
            "你好\tni3 hao3\t10\n绿色\tlv4 se4\t3\n略略\tlue4 lue4\t1\n坏行\tni3\t5\n东西\tdong1 xi5\tx\n",
        )
        .unwrap();
        let (rows, skipped) = load(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(skipped, 2);
        assert_eq!(rows[0].syllables, ["ni", "hao"]);
        assert_eq!(rows[0].tones, [3, 3]);
        assert_eq!(rows[2].syllables, ["lve", "lve"]);
    }

    fn entry(text: &str, syllables: &[&str], frequency: u32) -> LexiconEntry {
        LexiconEntry {
            text: text.to_owned(),
            syllables: syllables.iter().map(|s| (*s).to_owned()).collect(),
            frequency,
            tones: Vec::new(),
        }
    }

    fn keyed(entry: LexiconEntry) -> ((String, Vec<String>), LexiconEntry) {
        ((entry.text.clone(), entry.syllables.clone()), entry)
    }

    #[test]
    fn merges_into_base_and_new_words() {
        // 一千对读音相同的词：权重 w ↔ 词频 4w，拟合出来大约 ×4
        let shared: Vec<String> = (0..1000u32)
            .map(|i| {
                let a = char::from_u32(0x4e00 + i).unwrap();
                format!("{a}{a}")
            })
            .collect();
        let mut source = String::new();
        let mut entries = Entries::new();
        for (i, text) in shared.iter().enumerate() {
            let weight = i as u64 + 1;
            source.push_str(&format!("{text}\tyi1 yi1\t{weight}\n"));
            entries.extend([keyed(entry(text, &["yi", "yi"], 4 * weight as u32 + 3))]);
        }
        source.push_str("荨麻疹\txun2 ma2 zhen3\t5\n传说中的\tchuan2 shuo1 zhong1 de5\t800\n有代沟\tyou3 dai4 gou1\t73\n");
        entries.extend([keyed(entry(
            "传说中的",
            &["chuan", "shuo", "zhong", "di"],
            3561,
        ))]);
        let path =
            std::env::temp_dir().join(format!("qingjian-general-merge-{}.tsv", std::process::id()));
        std::fs::write(&path, source).unwrap();
        let merged = merge(&path, &HashSet::from(["有代沟".to_owned()]), &mut entries).unwrap();
        let general = &merged.general;
        std::fs::remove_file(path).unwrap();

        let key = |text: &str, syllables: &[&str]| {
            (
                text.to_owned(),
                syllables
                    .iter()
                    .map(|s| (*s).to_owned())
                    .collect::<Vec<_>>(),
            )
        };
        // 已有的词补声调、词频不动
        let first = &entries[&key(&shared[0], &["yi", "yi"])];
        assert_eq!((first.frequency, first.tones.as_slice()), (7, &[1, 1][..]));
        assert_eq!(
            general[&key("荨麻疹", &["xun", "ma", "zhen"])].tones,
            [2, 2, 3]
        );
        // 基础词库的读音留着，新读音降权进通用词库
        assert_eq!(
            entries[&key("传说中的", &["chuan", "shuo", "zhong", "di"])].frequency,
            3561
        );
        let disputed = &general[&key("传说中的", &["chuan", "shuo", "zhong", "de"])];
        let added = &merged.cleaned[&key("有代沟", &["you", "dai", "gou"])];
        assert_eq!((general.len(), merged.cleaned.len()), (2, 1));
        assert!(
            (280..=310).contains(&added.frequency),
            "{}",
            added.frequency
        );
        assert!(disputed.frequency < 800 * 4 / DISPUTED_READING_DIVISOR + 10);
        assert_eq!(added.tones, [3, 4, 1]);
    }

    #[test]
    fn fits_a_power_law() {
        let pairs: Vec<(u64, u32)> = (1..2000u64)
            .map(|w| (w, ((w as f64 + 1.0).powf(0.9) * 4.0 - 1.0).round() as u32))
            .collect();
        let scale = Scale::fit(&pairs).unwrap();
        assert!((scale.slope - 0.9).abs() < 0.01, "{scale:?}");
        assert!((scale.intercept - 4f64.ln()).abs() < 0.05, "{scale:?}");
        assert_eq!(scale.apply(0), 3);
        assert!(Scale::fit(&pairs[..10]).is_err());
    }
}
