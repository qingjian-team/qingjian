//! 学习译词的抽样生命周期与上屏一致性。

use std::sync::{Arc, Mutex};

use super::{MemoryVocabulary, engine};
use crate::{Language, Sense, Translation, TranslationDifficulty, Translator};

type Calls = Arc<Mutex<Vec<(TranslationDifficulty, Option<u64>)>>>;

struct SamplingTranslator {
    calls: Calls,
}

fn plain(text: impl Into<String>) -> Sense {
    Sense {
        part_of_speech: None,
        text: text.into(),
        reading: None,
        fresh: false,
    }
}

impl Translator for SamplingTranslator {
    fn language(&self) -> Language {
        Language::English
    }

    fn translate(&self, text: &str) -> Option<Translation> {
        (text == "开发").then(|| Translation::new(Language::English, vec![plain("develop")]))
    }

    fn translate_for_learning(
        &self,
        text: &str,
        difficulty: TranslationDifficulty,
        seed: Option<u64>,
    ) -> Option<Translation> {
        if text != "开发" {
            return None;
        }
        self.calls.lock().unwrap().push((difficulty, seed));
        Some(Translation::new(
            Language::English,
            vec![plain(seed.map_or_else(
                || "develop".to_owned(),
                |seed| format!("translation-{seed}"),
            ))],
        ))
    }
}

#[test]
fn repeated_annotation_and_typing_keep_the_same_translation_and_commit_it() {
    let calls = Calls::default();
    let mut engine = engine().with_translator(Box::new(SamplingTranslator {
        calls: calls.clone(),
    }));
    engine.set_translation_preferences(TranslationDifficulty::Advanced, 100);
    engine.set_input("kaifa");
    let mut list = engine.query().unwrap().candidates;
    engine.annotate(&mut list);
    let first = list
        .items
        .iter()
        .find(|c| c.text == "开发")
        .unwrap()
        .translation
        .clone();
    engine.annotate(&mut list);
    assert_eq!(
        list.items
            .iter()
            .find(|c| c.text == "开发")
            .unwrap()
            .translation,
        first
    );
    engine.push('z');
    let mut extended = engine.query().unwrap().candidates;
    engine.annotate(&mut extended);
    let candidate = extended
        .items
        .iter()
        .find(|c| c.text == "开发")
        .unwrap()
        .clone();
    assert_eq!(candidate.translation, first);
    assert_eq!(
        engine.commit_translation(&candidate, 0).unwrap(),
        first.unwrap().senses()[0].text
    );
    let calls = calls.lock().unwrap();
    assert!(calls.iter().all(
        |(difficulty, seed)| *difficulty == TranslationDifficulty::Advanced && seed.is_some()
    ));
    assert!(calls.windows(2).all(|pair| pair[0].1 == pair[1].1));
}

#[test]
fn frequency_changes_between_rounds_and_zero_preserves_fixed_translation() {
    let calls = Calls::default();
    let mut engine = engine().with_translator(Box::new(SamplingTranslator {
        calls: calls.clone(),
    }));
    engine.set_translation_preferences(TranslationDifficulty::All, 0);
    engine.set_input("kaifa");
    let mut list = engine.query().unwrap().candidates;
    engine.annotate(&mut list);
    assert_eq!(calls.lock().unwrap().last().unwrap().1, None);
    engine.set_translation_preferences(TranslationDifficulty::All, 50);
    for _ in 0..256 {
        engine.clear();
        engine.set_input("kaifa");
        let mut list = engine.query().unwrap().candidates;
        engine.annotate(&mut list);
    }
    let calls = calls.lock().unwrap();
    let random = calls.iter().filter(|(_, seed)| seed.is_some()).count();
    assert!((64..192).contains(&random), "50% frequency: {random}/256");
    let seeds: std::collections::HashSet<_> = calls.iter().filter_map(|(_, seed)| *seed).collect();
    assert_eq!(seeds.len(), random);
}

#[test]
fn random_frequency_is_clamped_and_does_not_change_candidate_order() {
    let calls = Calls::default();
    let mut engine = engine().with_translator(Box::new(SamplingTranslator {
        calls: calls.clone(),
    }));
    engine.set_translation_preferences(TranslationDifficulty::All, 255);
    engine.set_input("kaifa");
    let mut list = engine.query().unwrap().candidates;
    let texts: Vec<_> = list.items.iter().map(|c| c.text.clone()).collect();
    engine.annotate(&mut list);
    assert_eq!(
        texts,
        list.items
            .iter()
            .map(|c| c.text.clone())
            .collect::<Vec<_>>()
    );
    assert!(calls.lock().unwrap().iter().all(|(_, seed)| seed.is_some()));
}

#[test]
fn suspended_sessions_keep_their_sample_without_reusing_new_rounds() {
    let calls = Calls::default();
    let mut engine = engine().with_translator(Box::new(SamplingTranslator {
        calls: calls.clone(),
    }));
    engine.set_translation_preferences(TranslationDifficulty::All, 100);
    engine.set_input("kaifa");
    let mut list = engine.query().unwrap().candidates;
    engine.annotate(&mut list);
    let original = calls.lock().unwrap().last().unwrap().1;
    let mut session = crate::EngineSession::default();
    engine.swap_session(&mut session);
    engine.set_input("kaifa");
    let mut list = engine.query().unwrap().candidates;
    engine.annotate(&mut list);
    assert_ne!(original, calls.lock().unwrap().last().unwrap().1);
    engine.swap_session(&mut session);
    let mut list = engine.query().unwrap().candidates;
    engine.annotate(&mut list);
    assert_eq!(original, calls.lock().unwrap().last().unwrap().1);
}

#[test]
fn vocabulary_records_the_displayed_random_sense() {
    let book = Arc::new(Mutex::new(std::collections::HashMap::new()));
    let mut engine = engine()
        .with_translator(Box::new(SamplingTranslator {
            calls: Calls::default(),
        }))
        .with_vocabulary_tracker(Box::new(MemoryVocabulary(book.clone())));
    engine.set_translation_preferences(TranslationDifficulty::All, 100);
    engine.set_input("kaifa");
    let mut list = engine.query().unwrap().candidates;
    engine.annotate(&mut list);
    let candidate = list
        .items
        .iter()
        .find(|c| c.text == "开发")
        .unwrap()
        .clone();
    let displayed = candidate.translation.as_ref().unwrap().senses()[0]
        .text
        .clone();
    engine.note_displayed(list.items.iter());
    assert_eq!(
        engine.commit_translation(&candidate, 0),
        Some(displayed.clone())
    );
    let book = book.lock().unwrap();
    assert_eq!(book.get(&(Language::English, displayed)), Some(&(1, 1, 1)));
    assert!(!book.contains_key(&(Language::English, "develop".to_owned())));
}
