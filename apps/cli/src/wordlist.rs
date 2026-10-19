//! `--import-words`：导入只有词、没有拼音的词表，读音按参考词库（`--dict`）逐字派生。
//!
//! 判据与设置页的「导入词库」共用一套（`qingjian_dictionary::import::word_list`）：每个字取参考词库
//! 里的读音，多音字把每种组合各写一条（`美露莘` 同时得到 `mei lu shen` 与 `mei lu xin`）；字不在
//! 参考词库里的词、读音组合超上限的词跳过并列出。不给 `--import-to` 只打印预览，不写任何文件。

use std::path::{Path, PathBuf};

use qingjian_dictionary::{Dictionary, import};

use crate::args::Args;
use crate::error::CliError;

/// 预览里每类跳过原因最多列几个词。
const MAX_SKIPPED_SHOWN: usize = 10;

pub fn run(args: &Args, source: &Path) -> Result<(), CliError> {
    let reference = Dictionary::from_path(default_dict())?;
    let words = import::parse_words(&std::fs::read_to_string(source)?);
    let derived = import::derive(&words, &reference);
    let skipped = derived.report.unknown.len() + derived.report.overflowed.len();

    println!(
        "# 词表 {} 条：派生 {}、跳过 {}（字不在参考词库 {}、读音组合超上限 {}）",
        words.len(),
        derived.report.words,
        skipped,
        derived.report.unknown.len(),
        derived.report.overflowed.len()
    );
    println!("# 词\t拼音\t词频（同一个词多音字会占多行）");
    print!("{}", derived.tsv);
    print_skipped("字不在参考词库", &derived.report.unknown);
    print_skipped("读音组合超过上限", &derived.report.overflowed);

    match &args.import_to {
        Some(dir) => {
            let imported = import::import_word_list(source, dir, &reference)?;
            println!(
                "# 已导入「{}」到 {}，{} 条",
                imported.imported.name,
                imported.imported.path.display(),
                imported.imported.entries
            );
        }
        None => println!("# 预演：未给 --import-to，未落盘"),
    }
    Ok(())
}

/// 一类跳过的词列一行，最多 [`MAX_SKIPPED_SHOWN`] 个。
fn print_skipped(reason: &str, words: &[String]) {
    if words.is_empty() {
        return;
    }
    let shown = words.iter().take(MAX_SKIPPED_SHOWN).map(String::as_str);
    let rest = if words.len() > MAX_SKIPPED_SHOWN {
        " …"
    } else {
        ""
    };
    println!(
        "# 跳过（{reason}）：{}{rest}",
        shown.collect::<Vec<_>>().join("、")
    );
}

/// 与 `build_engine` 同一套缺省：`data/generated/dict.qj` 优先，退回仓库自带的样例。
fn default_dict() -> PathBuf {
    crate::args::default_data_file("dict.tsv")
}
