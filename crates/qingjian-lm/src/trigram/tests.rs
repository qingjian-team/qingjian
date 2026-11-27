use std::path::PathBuf;

use super::TrigramModel;

/// 测试用的临时文件，用完删掉。
struct TempFile(PathBuf);

impl TempFile {
    fn new(name: &str, bytes: &[u8]) -> Self {
        let path = std::env::temp_dir().join(format!(
            "qingjian-trigram-{}-{name}.ngram",
            std::process::id()
        ));
        std::fs::write(&path, bytes).unwrap();
        Self(path)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// 按 `.ngram` 格式拼一个小模型：词表（会按字节序排好）、一元 (log10 P, 回退)、二元 (前, 后, log10 P, 回退)、三元 (前前, 前, 后, log10 P)。
fn build(
    words: &[&str],
    unigrams: &[(f64, f64)],
    bigrams: &[(&str, &str, f64, f64)],
    trigrams: &[(&str, &str, &str, f64)],
    name: &str,
) -> TempFile {
    let mut sorted: Vec<(&str, (f64, f64))> = words
        .iter()
        .copied()
        .zip(unigrams.iter().copied())
        .collect();
    sorted.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    let id = |w: &str| sorted.iter().position(|(t, _)| *t == w).unwrap() as u32;
    let q = |v: f64| (v * 2048.0).round() as i16;
    let mut bi: Vec<(u32, u32, i16, i16)> = bigrams
        .iter()
        .map(|&(v, w, p, b)| (id(v), id(w), q(p), q(b)))
        .collect();
    bi.sort();
    let bi_index = |v: u32, w: u32| {
        bi.iter()
            .position(|&(a, b, _, _)| (a, b) == (v, w))
            .unwrap() as u32
    };
    let mut tri: Vec<(u32, u32, i16)> = trigrams
        .iter()
        .map(|&(u, v, w, p)| (bi_index(id(u), id(v)), id(w), q(p)))
        .collect();
    tri.sort();

    let mut out: Vec<u8> = Vec::new();
    let align = |out: &mut Vec<u8>| out.resize(out.len().div_ceil(8) * 8, 0);
    out.extend_from_slice(b"ZYNGRAM1");
    for n in [sorted.len(), bi.len(), tri.len(), 0] {
        out.extend_from_slice(&(n as u32).to_le_bytes());
    }
    let n1 = sorted.len() as u32;
    let starts = |keys: &[u32], count: u32| -> Vec<u32> {
        (0..=count)
            .map(|k| keys.iter().filter(|&&key| key < k).count() as u32)
            .collect()
    };
    let mut offset = 0u32;
    let mut offsets = vec![0u32];
    for (w, _) in &sorted {
        offset += w.len() as u32;
        offsets.push(offset);
    }
    let sections: Vec<Vec<u8>> = vec![
        offsets.iter().flat_map(|v| v.to_le_bytes()).collect(),
        sorted
            .iter()
            .flat_map(|(w, _)| w.as_bytes().to_vec())
            .collect(),
        sorted
            .iter()
            .flat_map(|(_, (p, _))| q(*p).to_le_bytes())
            .collect(),
        sorted
            .iter()
            .flat_map(|(_, (_, b))| q(*b).to_le_bytes())
            .collect(),
        starts(&bi.iter().map(|b| b.0).collect::<Vec<_>>(), n1)
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect(),
        bi.iter().flat_map(|b| b.1.to_le_bytes()).collect(),
        bi.iter().flat_map(|b| b.2.to_le_bytes()).collect(),
        bi.iter().flat_map(|b| b.3.to_le_bytes()).collect(),
        starts(
            &tri.iter().map(|t| t.0).collect::<Vec<_>>(),
            bi.len() as u32,
        )
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect(),
        tri.iter().flat_map(|t| t.1.to_le_bytes()).collect(),
        tri.iter().flat_map(|t| t.2.to_le_bytes()).collect(),
    ];
    for section in sections {
        align(&mut out);
        out.extend_from_slice(&section);
    }
    TempFile::new(name, &out)
}

fn model(name: &str) -> (TempFile, TrigramModel) {
    let file = build(
        &["<unk>", "灾害", "在", "还", "这次", "是"],
        &[
            (-6.0, 0.0),
            (-4.0, -0.5),
            (-2.0, -0.3),
            (-2.5, -0.3),
            (-3.5, -0.2),
            (-1.8, -0.4),
        ],
        &[
            ("这次", "灾害", -1.0, -0.2),
            ("在", "还", -2.0, 0.0),
            ("灾害", "是", -0.8, 0.0),
        ],
        &[("这次", "灾害", "是", -0.3)],
        name,
    );
    let model = TrigramModel::open(file.path()).unwrap();
    (file, model)
}

#[test]
fn backs_off_from_trigram_to_bigram_to_unigram() {
    let (_file, model) = model("backoff");
    let id = |w| model.word(w).unwrap();
    let close = |a: f64, b: f64| (a - b).abs() < 1e-3;
    // 三元命中
    assert!(close(
        model.log10(Some(id("这次")), Some(id("灾害")), id("是")),
        -0.3
    ));
    // 二元命中、没有前前词
    assert!(close(model.log10(None, Some(id("这次")), id("灾害")), -1.0));
    // 二元没有：前词的回退权重 + 一元
    assert!(close(
        model.log10(None, Some(id("还")), id("是")),
        -0.3 + -1.8
    ));
    // (还, 是) 不成二元：三元历史不存在，不加回退权重直接退到 P(灾害 | 是) → 一元回退
    assert!(close(
        model.log10(Some(id("还")), Some(id("是")), id("灾害")),
        -0.4 + -4.0
    ));
    assert_eq!(model.word("没有这个词"), None);
}

#[test]
fn a_common_word_after_its_context_beats_two_loose_characters() {
    let (_file, model) = model("score");
    let scores = model.score_texts("这次", &["灾害", "在还"]);
    assert!(scores[0] > scores[1], "{scores:?}");
    // ln P：二元 (这次, 灾害) 是 log10 -1.0
    assert!((scores[0] + std::f64::consts::LN_10).abs() < 1e-3);
}

#[test]
fn rejects_files_that_are_not_ngrams() {
    let file = TempFile::new("bad", b"not an ngram file at all");
    assert!(TrigramModel::open(file.path()).is_err());
}

#[test]
fn as_a_language_model_uses_the_earlier_word_and_ignores_unknown_history() {
    use qingjian_core::sentence::LanguageModel;

    let (_file, model) = model("lattice");
    let ln = |log10: f64| log10 * std::f64::consts::LN_10;
    let close = |a: Option<f64>, b: f64| (a.unwrap() - b).abs() < 1e-3;
    assert!(close(
        model.log_prob_trigram(Some("这次"), Some("灾害"), "是"),
        ln(-0.3)
    ));
    assert!(close(model.log_prob(Some("灾害"), "是"), ln(-0.8)));
    // 前词不在词表：当没有前文，前前词也一并不用
    assert!(close(
        model.log_prob_trigram(Some("这次"), Some("没有这个词"), "是"),
        ln(-1.8)
    ));
    assert_eq!(model.log_prob(None, "没有这个词"), None);
}

#[test]
fn vocabulary_lists_every_word_with_its_unigram() {
    let (_file, model) = model("vocabulary");
    let words: Vec<(&str, f64)> = model.vocabulary().collect();
    assert_eq!(words.len(), 6);
    let (_, log10) = words.iter().find(|(text, _)| *text == "这次").unwrap();
    assert!((log10 + 3.5).abs() < 1e-3, "{log10}");
}

#[test]
fn counts_and_size_come_from_the_file() {
    let (_file, model) = model("counts");
    assert_eq!(model.unigram_count(), 6);
    assert_eq!(model.bigram_count(), 3);
    assert_eq!(model.trigram_count(), 1);
    assert!(model.bytes() > 24);
}

#[test]
fn an_empty_context_scores_the_plain_unigram() {
    let (_file, model) = model("plain");
    let scores = model.score_texts("", &["是"]);
    assert!(
        (scores[0] + 1.8 * std::f64::consts::LN_10).abs() < 1e-3,
        "{scores:?}"
    );
}

#[test]
fn only_the_last_line_of_the_context_counts() {
    let (_file, model) = model("last-line");
    let plain = model.score_texts("这次", &["灾害"]);
    let prefixed = model.score_texts("另一行\n这次", &["灾害"]);
    assert!(
        (plain[0] - prefixed[0]).abs() < 1e-9,
        "{plain:?} {prefixed:?}"
    );
}

#[test]
fn the_context_window_keeps_the_last_twelve_characters() {
    let (_file, model) = model("window");
    let thirteen: String = "是".repeat(11) + "这次";
    let fifteen: String = "是".repeat(13) + "这次";
    let shorter = model.score_texts(&thirteen, &["灾害"]);
    let longer = model.score_texts(&fifteen, &["灾害"]);
    assert!(
        (shorter[0] - longer[0]).abs() < 1e-9,
        "{shorter:?} {longer:?}"
    );
}

#[test]
fn no_candidates_yield_no_scores() {
    let (_file, model) = model("no-texts");
    assert!(model.score_texts("这次", &[]).is_empty());
}

#[test]
fn unknown_characters_go_through_the_unknown_word() {
    let (_file, model) = model("unknown");
    let scores = model.score_texts("", &["嗯"]);
    assert!(
        (scores[0] + 6.0 * std::f64::consts::LN_10).abs() < 1e-3,
        "{scores:?}"
    );
}

#[test]
fn a_word_beats_a_pair_of_unknown_characters() {
    let (_file, model) = model("compare");
    let scores = model.score_texts("这次", &["灾害", "灾嗯"]);
    assert!(scores[0] > scores[1], "{scores:?}");
}

#[test]
fn rejects_empty_short_and_empty_vocabulary_files() {
    let empty = TempFile::new("empty", b"");
    assert!(TrigramModel::open(empty.path()).is_err());
    let magic_only = TempFile::new("magic-only", b"ZYNGRAM1");
    assert!(TrigramModel::open(magic_only.path()).is_err());
    let mut header = Vec::from(*b"ZYNGRAM1");
    for _ in 0..4 {
        header.extend_from_slice(&0u32.to_le_bytes());
    }
    let no_words = TempFile::new("no-words", &header);
    assert!(TrigramModel::open(no_words.path()).is_err());
}
