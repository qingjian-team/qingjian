//! `--import-words`：导入只有词、没有拼音的词表，读音按参考词库取。
//!
//! 判据与设置页的「导入词库」共用一套（`qingjian_dictionary::import::word_list`）：词表里写明的读音优先，
//! 其次整词词条（`建设银行`），再次按词库最长匹配切分（`中华人民共和国`），切不动的单字取词频最高的读音。
//! 一个词只写一条读音组合；字不在参考词库里的词跳过并列出。参考词库是 `--dict`（缺省随包基础词库）
//! 加 `--extra-dict`，再加随包的领域词库（`assets/lexicon/dicts/*.tsv`）。
//! 不给 `--import-to` 只打印预览，不写任何文件。

use std::path::{Path, PathBuf};

use qingjian_dictionary::{Dictionary, import};

use crate::args::Args;
use crate::error::CliError;

/// 预览里每类跳过原因最多列几个词。
const MAX_SKIPPED_SHOWN: usize = 10;

/// 随包领域词库目录（`dict-convert` 产出的 `词\t音节\t词频`）。
const DOMAIN_DICTS: &str = "assets/lexicon/dicts";

pub fn run(args: &Args, source: &Path) -> Result<(), CliError> {
    let references = load_references(args)?;
    let borrowed: Vec<&Dictionary> = references.iter().collect();
    let words = import::parse_words(&std::fs::read_to_string(source)?);
    let derived = import::derive(&words, &borrowed);

    println!(
        "# 词表 {} 条：写入 {}、跳过 {}（字不在参考词库）",
        words.len(),
        derived.report.words,
        derived.report.unknown.len()
    );
    println!("# 词\t拼音\t词频");
    print!("{}", derived.tsv);
    print_skipped("字不在参考词库", &derived.report.unknown);

    match &args.import_to {
        Some(dir) => {
            let imported = import::import_word_list(source, dir, &borrowed)?;
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

/// 参考词库：`--dict`（缺省随包基础词库）、`--extra-dict`，以及随包领域词库。
fn load_references(args: &Args) -> Result<Vec<Dictionary>, CliError> {
    let path = args.dict.clone().unwrap_or_else(default_dict);
    let mut references = vec![Dictionary::from_path(path)?];
    for extra in &args.extra_dict {
        references.push(Dictionary::from_path(extra)?);
    }
    if let Ok(entries) = std::fs::read_dir(DOMAIN_DICTS) {
        let mut paths: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "tsv"))
            .collect();
        paths.sort();
        for path in paths {
            references.push(Dictionary::from_path(path)?);
        }
    }
    Ok(references)
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
