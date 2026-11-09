//! 语音上屏：繁体转换、输入统计、空文字。

use super::*;

use crate::traditional::TraditionalVariant;

#[test]
fn voice_text_is_converted_when_traditional_output_is_on() {
    let mut engine = engine();
    assert_eq!(engine.commit_voice("开发者大会"), "开发者大会");
    engine.set_traditional(TraditionalVariant::Taiwan);
    assert_eq!(engine.commit_voice("开发者大会"), "開發者大會");
}

/// 语音照记字数与上屏次数，但不算选词，也不拿之前打字的键数充数。
#[test]
fn voice_is_metered_but_not_counted_as_a_choice() {
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let mut engine = engine().with_usage_meter(Box::new(MemoryMeter(recorded.clone())));
    engine.set_input("kaif");
    engine.clear();
    engine.commit_voice("开饭了 ok");
    let recorded = recorded.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    let usage = &recorded[0];
    assert_eq!(usage.hanzi, 3);
    assert_eq!(usage.english_words, 1);
    assert_eq!(usage.commits, 1);
    assert_eq!(usage.choices, 0);
    assert_eq!(usage.keys, 0);
}

#[test]
fn blank_voice_text_commits_nothing() {
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let mut engine = engine().with_usage_meter(Box::new(MemoryMeter(recorded.clone())));
    assert_eq!(engine.commit_voice("  "), "");
    assert!(recorded.lock().unwrap().is_empty());
}
