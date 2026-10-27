//! 形码码表：Rime `.dict.yaml`（极点 86 五笔等）→ `词\t编码\t词频`。
//!
//! 码表自带的第二列（Rime 里是权重）**不用**：那是码表顺序，不是语料词频，直接拿来排序会让同一个词
//! 在形码下和在拼音下排得不一样。词频从青简词库（`词\t拼音\t词频`）按**词面**交叉回填：
//! 词库里有的用语料词频（与拼音方案同一把尺子，释义兜底与生词识别也对得上），
//! 没有的给 [`UNKNOWN_FREQUENCY`]——语料里一次都没出现过，本来就该排在同编码的已知词后面。
//!
//! 词组编码（二字词 2+2、三字词 1+1+1）由上游码表给全，这里不做取码推导，见 `docs/plan/wubi.md`。

use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use qingjian_dictionary::Dictionary;
use qingjian_dictionary::import::{looks_like_rime, to_tsv};

use crate::error::ConvertError;

/// 词库里查不到的词给的词频。
pub const UNKNOWN_FREQUENCY: u32 = 1;

/// 转换结果，打日志用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Converted {
    /// 写出的词条数。
    pub entries: usize,

    /// 其中词频来自语料的条数。
    pub with_frequency: usize,

    /// 词库里查不到、拿了 [`UNKNOWN_FREQUENCY`] 的条数。占比高说明码表与词库的词面写法对不上，要查。
    pub unknown: usize,
}

/// 读 Rime 码表写出青简形码 TSV，词频从 `frequencies`（青简词库 TSV，可给多本）交叉回填。
pub fn convert(
    input: &Path,
    frequencies: &[PathBuf],
    output: &Path,
) -> Result<Converted, ConvertError> {
    let source = std::fs::read_to_string(input)?;
    if !looks_like_rime(&source) {
        return Err(ConvertError::Format {
            path: input.to_owned(),
            line: 0,
            reason: "expected a Rime .dict.yaml (a `---` header or a `name:` line)".to_owned(),
        });
    }
    // 基础词库加随包的领域词库：词的语料词频与它收在哪一本里无关，同一个词取最大的那份
    let dictionaries: Vec<Dictionary> = frequencies
        .iter()
        .map(Dictionary::from_path)
        .collect::<Result<_, _>>()?;
    let mut table: HashMap<&str, u32> = HashMap::new();
    for dictionary in &dictionaries {
        for hit in dictionary.entries() {
            let slot = table.entry(hit.text).or_insert(hit.frequency);
            *slot = (*slot).max(hit.frequency);
        }
    }

    let tsv = to_tsv(&source).tsv;
    let mut entries: Vec<(String, String, u32, usize)> = Vec::new();
    for (index, raw) in tsv.lines().enumerate() {
        // 只去尾部空白：词字段可能是空的（源表里 全角空格 那条以制表符开头），
        // 整行 trim 会把开头的制表符吃掉、后面的列整体左移，编码被当成词
        let line = raw.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(word), Some(code)) = (fields.next(), fields.next()) else {
            return Err(ConvertError::Format {
                path: input.to_owned(),
                line: index + 1,
                reason: "expected `word<TAB>code`".to_owned(),
            });
        };
        let (word, code) = (word.trim(), code.trim().to_ascii_lowercase());
        if word.is_empty() || code.is_empty() {
            continue;
        }
        let weight = table.get(word).copied().unwrap_or(UNKNOWN_FREQUENCY);
        entries.push((word.to_owned(), code, weight, index));
    }
    // 单字母编码保留源码表顺序，避免一级简码被回填词频覆盖；其他编码仍按词频排序。
    entries.sort_by(|a, b| {
        a.1.cmp(&b.1)
            .then_with(|| {
                if a.1.len() == 1 {
                    a.3.cmp(&b.3)
                } else {
                    b.2.cmp(&a.2)
                }
            })
            .then_with(|| a.0.cmp(&b.0))
    });
    entries.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);

    let unknown = entries
        .iter()
        .filter(|(_, _, frequency, _)| *frequency == UNKNOWN_FREQUENCY)
        .count();
    let mut writer = BufWriter::new(std::fs::File::create(output)?);
    writeln!(
        writer,
        "# 形码码表：词\t编码\t词频。词频由青简词库按词面回填（不是码表自带的权重），出处见 assets/schemes/wubi86/README.md"
    )?;
    for (word, code, frequency, _) in &entries {
        writeln!(writer, "{word}\t{code}\t{frequency}")?;
    }
    writer.flush()?;
    Ok(Converted {
        entries: entries.len(),
        with_frequency: entries.len() - unknown,
        unknown,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &str = r#"---
name: wubi86
...
的	r	100
一	ggll	90
开	ga	80
开发	gant	70
不给	gqt	60
"#;

    /// 词库里有 的 / 开 / 开发 / 一，码表里的 不给 不在其中，用来验证按词面回填与兜底。
    const DICT: &str = "的\tde\t5000\n开\tkai\t800\n开发\tkai fa\t900\n一\tyi\t4000\n";

    /// 每个用例一个自己的目录：`cargo test` 并行跑，共用目录会互相覆盖。
    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("qingjian-wubi-{name}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn convert_to_strings(name: &str, table: &str, dict: &str) -> (Vec<String>, Converted) {
        let dir = scratch(name);
        std::fs::create_dir_all(&dir).unwrap();
        let table_path = dir.join("table.dict.yaml");
        let dict_path = dir.join("dict.tsv");
        let out_path = dir.join("wubi86.tsv");
        std::fs::write(&table_path, table).unwrap();
        std::fs::write(&dict_path, dict).unwrap();
        let converted = convert(&table_path, std::slice::from_ref(&dict_path), &out_path).unwrap();
        let lines: Vec<String> = std::fs::read_to_string(&out_path)
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with('#'))
            .map(str::to_owned)
            .collect();
        (lines, converted)
    }

    #[test]
    fn fills_frequency_by_word_and_sorts_by_code() {
        let (lines, converted) = convert_to_strings("fills", TABLE, DICT);
        assert_eq!(
            lines,
            [
                "开\tga\t800",
                "开发\tgant\t900",
                "一\tggll\t4000",
                "不给\tgqt\t1",
                "的\tr\t5000",
            ]
        );
        assert_eq!(
            converted,
            Converted {
                entries: 5,
                with_frequency: 4,
                unknown: 1,
            }
        );
    }

    #[test]
    fn preserves_source_order_for_single_letter_codes() {
        let (lines, _) = convert_to_strings(
            "single-letter-order",
            "---\nname: t\n...\n以\tc\t1\n又\tc\t2\n",
            "以\tyi\t1\n又\tyou\t999999\n",
        );
        assert_eq!(lines, ["以\tc\t1", "又\tc\t999999"]);
    }

    #[test]
    fn words_missing_from_the_dictionary_get_the_floor_frequency() {
        let (lines, converted) =
            convert_to_strings("floor", TABLE, "差不多的词\tcha bu duo de ci\t9\n");
        assert!(lines.iter().all(|l| l.ends_with("\t1")));
        assert_eq!(converted.unknown, 5);
        assert_eq!(converted.with_frequency, 0);
    }

    #[test]
    fn keeps_the_highest_frequency_when_a_word_appears_twice() {
        let (lines, _) =
            convert_to_strings("dupes", "---\nname: t\n...\n开发\tgant\n开发\tgant\n", DICT);
        assert_eq!(lines, ["开发\tgant\t900"]);
    }

    #[test]
    fn skips_rows_whose_word_field_is_empty() {
        // 极点码表里 全角空格 那条就是这个形状：词字段是空的全角空格，行以制表符开头。
        // 整行 trim 会把制表符吃掉，变成「编码 cokg 的词频 10」这种垃圾条目。
        let (lines, converted) = convert_to_strings(
            "empty-word",
            "---\nname: t\n...\n\tcokg\t10\t全角空格\n开\tga\t80\n",
            DICT,
        );
        assert_eq!(lines, ["开\tga\t800"]);
        assert_eq!(converted.entries, 1);
    }

    #[test]
    fn rejects_files_that_are_not_rime_dictionaries() {
        let dir = scratch("not-rime");
        let table = dir.join("plain.tsv");
        let dict_path = dir.join("dict.tsv");
        std::fs::write(&table, "开\tga\t800\n").unwrap();
        std::fs::write(&dict_path, DICT).unwrap();
        assert!(
            convert(
                &table,
                std::slice::from_ref(&dict_path),
                &dir.join("out.tsv")
            )
            .is_err()
        );
    }

    #[test]
    fn takes_the_highest_frequency_across_dictionaries() {
        // 基础词库没有 葡萄牙，领域词库有：两本都传进去就该命中，取大的那份
        let dir = scratch("multi");
        let table_path = dir.join("table.dict.yaml");
        let base_path = dir.join("dict.tsv");
        let domain_path = dir.join("places.tsv");
        let out_path = dir.join("wubi86.tsv");
        std::fs::write(&table_path, "---\nname: t\n...\n葡萄牙\taaah\t20\n").unwrap();
        std::fs::write(&base_path, "开\tkai\t800\n").unwrap();
        std::fs::write(&domain_path, "葡萄牙\tpu tao ya\t300\n").unwrap();
        let converted = convert(&table_path, &[base_path, domain_path], &out_path).unwrap();
        assert_eq!(converted.with_frequency, 1);
        assert_eq!(
            std::fs::read_to_string(&out_path)
                .unwrap()
                .lines()
                .filter(|l| !l.starts_with('#'))
                .collect::<Vec<_>>(),
            ["葡萄牙\taaah\t300"]
        );
    }
}
