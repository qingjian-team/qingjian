use std::sync::Arc;
use std::time::{Duration, Instant};

use qingjian_dictionary::Dictionary;

use super::*;

/// 假选词模型：每条都盖住整段，偏爱某个文本。
struct Prefers(&'static str);

impl ChoiceModel for Prefers {
    fn align(&self, _keys: &str, _syllables: &[String]) -> Option<Vec<u32>> {
        Some(vec![1])
    }

    fn letter_units(&self, _keys: &str) -> Vec<u32> {
        vec![1]
    }

    fn score(&self, _context: &str, _typed: &[u32], texts: &[&str]) -> Vec<f32> {
        texts
            .iter()
            .map(|t| if *t == self.0 { 30.0 } else { 0.0 })
            .collect()
    }
}

fn first_texts(engine: &Engine) -> Vec<String> {
    let query = engine.query().unwrap();
    query
        .candidates
        .items
        .iter()
        .take(2)
        .map(|c| c.text.clone())
        .collect()
}

#[test]
fn choice_waits_for_the_shell_then_moves_the_pick_up() {
    let mut engine =
        Engine::new(Dictionary::parse("开发\tkai fa\t9000\n开罚\tkai fa\t10\n").unwrap());
    engine.set_choice_model(Some(Arc::new(Prefers("开罚"))));
    for c in "kaifa".chars() {
        engine.push(c);
    }
    assert_eq!(first_texts(&engine), ["开发", "开罚"]);
    assert!(engine.rescoring_pending());
    assert!(engine.request_rescoring());
    assert!(!engine.rescoring_pending());
    let started = Instant::now();
    while !engine.poll_rescoring() {
        assert!(started.elapsed() < Duration::from_secs(5), "后台没回结果");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(first_texts(&engine), ["开罚", "开发"]);
    assert!(!engine.rescoring_pending());
    let note = engine.last_choice.get().expect("排过这页");
    assert_eq!((note.from, note.ctx, note.app), (1, 0, false));
}
