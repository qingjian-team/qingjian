//! 对着仓库里的产品数据查：重新生成 `dict.tsv` 时 `--extra-words` 少给一个文件，那批词会悄悄没了。

use std::collections::HashSet;
use std::path::PathBuf;

use super::corpus;

/// QINGJIAN.md 第 4 步那条 `lexicon` 要并入的全部额外词文件。
const EXTRA_WORDS: [&str; 5] = [
    "mined_words.tsv",
    "phrases.tsv",
    "brand.tsv",
    "domain_words.tsv",
    "mixed_words.tsv",
];

fn lexicon_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/lexicon")
}

/// 基础词库与拆出去的领域词库里的全部词。
fn generated_words() -> HashSet<String> {
    let dir = lexicon_dir();
    let mut files = vec![dir.join("dict.tsv")];
    for entry in std::fs::read_dir(dir.join("dicts")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "tsv") {
            files.push(path);
        }
    }
    let mut words = HashSet::new();
    for path in files {
        for line in std::fs::read_to_string(&path).unwrap().lines() {
            if let Some(text) = line.split('\t').next().filter(|_| !line.starts_with('#')) {
                words.insert(text.to_owned());
            }
        }
    }
    words
}

#[test]
fn every_extra_word_file_made_it_into_the_generated_dictionary() {
    let words = generated_words();
    for name in EXTRA_WORDS {
        let missing: Vec<String> = corpus::extra_words(&lexicon_dir().join(name))
            .unwrap()
            .into_iter()
            .map(|word| word.text)
            .filter(|text| !words.contains(text))
            .collect();
        assert!(
            missing.is_empty(),
            "{name} 里 {} 个词不在 dict.tsv 与 dicts/ 里（重新生成时漏了 --extra-words {name}？）：{:?}",
            missing.len(),
            &missing[..missing.len().min(10)]
        );
    }
}
