//! 释义表的解析、路径与分层查找；落盘往返在 `personal_glossary` 的测试里。

use std::path::Path;

use qingjian_core::{Language, Sense, Translation, Translator};

use crate::{Glossary, GlossaryError, LayeredTranslator, PersonalGlossary, downloaded_glossary};

fn one_sense(language: Language, text: &str) -> Translation {
    Translation::new(
        language,
        vec![Sense {
            part_of_speech: None,
            text: text.to_owned(),
            reading: None,
            pronunciation: None,
            fresh: false,
        }],
    )
}

#[test]
fn empty_glossary_has_no_entries() {
    let glossary = Glossary::empty(Language::English);
    assert!(glossary.is_empty());
    assert_eq!(glossary.len(), 0);
    assert!(glossary.translate("词").is_none());
}

#[test]
fn parse_skips_comments_and_blank_lines() {
    let glossary = Glossary::parse(
        Language::English,
        "# 注释\n\n开发\tv. develop\tn. development\n",
    )
    .unwrap();
    assert_eq!(glossary.len(), 1);
    assert!(!glossary.is_empty());
}

#[test]
fn parse_keeps_at_most_two_senses() {
    let glossary = Glossary::parse(
        Language::English,
        "开发\tv. develop\tn. development\tn. exploitation\n",
    )
    .unwrap();
    assert_eq!(glossary.translate("开发").unwrap().senses().len(), 2);
}

#[test]
fn parse_requires_a_word_and_a_sense() {
    assert!(Glossary::parse(Language::English, "\t译文").is_err());
    assert!(Glossary::parse(Language::English, "词").is_err());
}

#[test]
fn parse_reports_the_offending_line() {
    let error = Glossary::parse(Language::English, "好\t译\n\n坏\t").unwrap_err();
    assert!(
        matches!(error, GlossaryError::Line { line: 3, .. }),
        "{error:?}"
    );
}

#[test]
fn duplicate_words_keep_the_last_line() {
    let glossary = Glossary::parse(Language::English, "词\t第一\n词\t第二\n").unwrap();
    assert_eq!(glossary.translate("词").unwrap().senses()[0].text, "第二");
}

#[test]
fn translations_carry_the_glossary_language() {
    let glossary = Glossary::parse(Language::Japanese, "私\t私|わたし\n").unwrap();
    assert_eq!(
        glossary.translate("私").unwrap().language,
        Language::Japanese
    );
}

#[test]
fn downloaded_glossaries_live_under_the_user_dir() {
    let path = downloaded_glossary(Path::new("/data/user"), Language::English);
    assert!(path.starts_with("/data/user"), "{}", path.display());
    assert!(path.to_string_lossy().contains("glossaries"));
    assert_eq!(path.extension().unwrap(), "qj");
    assert_ne!(
        path,
        downloaded_glossary(Path::new("/data/user"), Language::Japanese)
    );
}

#[test]
fn personal_glossary_inserts_and_translates() {
    let mut glossary = PersonalGlossary::in_memory(Language::English);
    assert!(glossary.is_empty());
    glossary.insert("词", one_sense(Language::English, "word"));
    assert_eq!(glossary.len(), 1);
    assert_eq!(glossary.translate("词").unwrap().senses()[0].text, "word");
}

#[test]
fn personal_glossary_rejects_other_languages_and_empty_senses() {
    let mut glossary = PersonalGlossary::in_memory(Language::English);
    glossary.insert("词", one_sense(Language::Japanese, "word"));
    assert_eq!(glossary.len(), 0);
    glossary.insert("词", Translation::new(Language::English, Vec::new()));
    assert_eq!(glossary.len(), 0);
}

#[test]
fn personal_glossary_overwrites_the_same_word() {
    let mut glossary = PersonalGlossary::in_memory(Language::English);
    glossary.insert("词", one_sense(Language::English, "一"));
    glossary.insert("词", one_sense(Language::English, "二"));
    assert_eq!(glossary.len(), 1);
    assert_eq!(glossary.translate("词").unwrap().senses()[0].text, "二");
}

#[test]
fn layered_translator_prefers_the_personal_table() {
    let bundled = Glossary::parse(Language::English, "词\t随包\n").unwrap();
    let mut personal = PersonalGlossary::in_memory(Language::English);
    personal.insert("词", one_sense(Language::English, "个人"));
    let translator = LayeredTranslator::new(bundled, personal);
    assert_eq!(translator.translate("词").unwrap().senses()[0].text, "个人");
}

#[test]
fn layered_translator_falls_back_to_the_bundled_table() {
    let bundled = Glossary::parse(Language::English, "词\t随包\n").unwrap();
    let translator =
        LayeredTranslator::new(bundled, PersonalGlossary::in_memory(Language::English));
    assert_eq!(translator.translate("词").unwrap().senses()[0].text, "随包");
    assert!(translator.translate("没有").is_none());
}

#[test]
fn layered_translator_reports_both_sizes() {
    let bundled = Glossary::parse(Language::English, "词\t随包\n").unwrap();
    let mut personal = PersonalGlossary::in_memory(Language::English);
    personal.insert("另一个", one_sense(Language::English, "个人"));
    let translator = LayeredTranslator::new(bundled, personal);
    assert_eq!(translator.len(), 1);
    assert_eq!(translator.personal_len(), 1);
    assert!(!translator.is_empty());
}

#[test]
fn layered_translator_learns_into_the_personal_table() {
    let bundled = Glossary::parse(Language::English, "词\t随包\n").unwrap();
    let mut translator =
        LayeredTranslator::new(bundled, PersonalGlossary::in_memory(Language::English));
    translator.learn("新词", one_sense(Language::English, "new"));
    assert_eq!(translator.personal_len(), 1);
    assert_eq!(
        translator.translate("新词").unwrap().senses()[0].text,
        "new"
    );
}
