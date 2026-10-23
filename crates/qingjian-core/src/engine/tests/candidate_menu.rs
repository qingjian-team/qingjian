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
