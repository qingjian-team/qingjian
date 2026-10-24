use super::*;
use crate::CustomPhrase;

fn learning_engine() -> Engine {
    engine().with_learner(Box::new(CountingLearner(HashMap::new())))
}

fn first_texts(engine: &Engine, n: usize) -> Vec<String> {
    texts_of(engine).into_iter().take(n).collect()
}

#[test]
fn each_demotion_moves_the_word_one_place_back_until_undone() {
    let mut e = learning_engine();
    e.set_input("kaif");
    let before = first_texts(&e, 3);
    let top = e.query().unwrap().candidates.items[0].clone();

    e.demote(&top);
    assert_eq!(
        first_texts(&e, 3),
        [before[1].as_str(), &before[0], &before[2]]
    );
    e.demote(&top);
    assert_eq!(e.demotion(&top), 2);
    assert_eq!(
        first_texts(&e, 3),
        [before[1].as_str(), &before[2], &before[0]]
    );

    // 只管这个输入串
    e.set_input("xi'an");
    e.set_input("kaif");
    assert!(e.undemote(&top));
    assert_eq!(first_texts(&e, 3), before);
}

#[test]
fn demotion_works_in_private_input() {
    let mut e = learning_engine();
    e.set_private(true);
    e.set_input("kaif");
    let top = e.query().unwrap().candidates.items[0].clone();
    e.demote(&top);
    assert_eq!(e.demotion(&top), 1);
}

#[test]
fn phrases_are_not_demoted_and_pins_need_a_plain_code() {
    let mut e = learning_engine();
    e.set_custom_phrases(vec![CustomPhrase::new("kaif", "开饭")])
        .unwrap();
    e.set_input("kaif");
    let items = e.query().unwrap().candidates.items;
    assert!(!e.demotable(&items[0]));
    assert!(e.demotable(&items[1]));
    assert_eq!(
        e.pin_target(&items[1]),
        Some(("kaif".to_owned(), items[1].text.clone()))
    );
    e.set_input("xi'an");
    let items = e.query().unwrap().candidates.items;
    assert_eq!(e.pin_target(&items[0]), None);
}

#[test]
fn edited_translation_overrides_the_existing_gloss() {
    let mut e = engine().with_translator(Box::new(LearningTranslator::default()));
    e.set_input("kaifa");
    let candidate = e
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|candidate| candidate.text == "开发")
        .unwrap();

    assert!(e.set_candidate_translation(&candidate, "software development"));
    assert_eq!(
        e.candidate_translation("开发").unwrap().senses()[0].text,
        "software development"
    );
    assert_eq!(
        e.candidate_translation("开发").unwrap().senses()[0].part_of_speech,
        Some(PartOfSpeech::Verb)
    );
}

#[test]
fn editing_the_first_translation_keeps_the_second_sense_and_metadata() {
    let original = Translation::new(
        Language::English,
        vec![
            Sense {
                part_of_speech: Some(PartOfSpeech::Adjective),
                text: "first".to_owned(),
                reading: Some("reading".to_owned()),
                fresh: true,
            },
            Sense {
                part_of_speech: Some(PartOfSpeech::Noun),
                text: "second".to_owned(),
                reading: None,
                fresh: false,
            },
        ],
    );
    let mut translator = LearningTranslator::default();
    translator.learn("测试", original);
    let mut e = engine().with_translator(Box::new(translator));

    assert!(e.set_translation("测试", "replacement"));
    let translation = e.candidate_translation("测试").unwrap();
    assert_eq!(translation.senses().len(), 2);
    assert_eq!(translation.senses()[0].text, "replacement");
    assert_eq!(
        translation.senses()[0].part_of_speech,
        Some(PartOfSpeech::Adjective)
    );
    assert_eq!(translation.senses()[0].reading.as_deref(), Some("reading"));
    assert!(!translation.senses()[0].fresh);
    assert_eq!(translation.senses()[1].text, "second");
    assert_eq!(
        translation.senses()[1].part_of_speech,
        Some(PartOfSpeech::Noun)
    );
}

#[test]
fn model_translation_targets_the_current_learning_language() {
    let submitted = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let predictor = EchoPredictor {
        submitted: submitted.clone(),
        replies: Vec::new(),
        sentence: false,
    };
    let mut e = engine()
        .with_translator(Box::new(LearningTranslator::default()))
        .with_predictor(Box::new(predictor));

    e.request_candidate_translation("开发").unwrap();
    let requests = submitted.borrow();
    let request = requests.last().unwrap();
    assert_eq!(request.kind, PredictionKind::CandidateTranslation);
    assert_eq!(request.text, "开发");
    assert_eq!(request.target_language, "en");
}
