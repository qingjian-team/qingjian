//! 拼音行按高亮候选显示带调拼音（`[general] show_tones`）。

use super::*;

/// `input` 下高亮 `text` 时候选窗口拼音行的文字与光标，以及应用输入框里的 marked text。
fn window_line(engine: &mut Engine, input: &str, text: &str) -> (String, usize, String) {
    engine.set_input(input);
    let query = engine.query().unwrap();
    let candidate = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == text)
        .unwrap()
        .clone();
    let mut query = query;
    query.follow(Some(&candidate));
    let toned = engine.toned_syllables(&candidate);
    let (segments, cursor) = query.window_segments(toned.as_deref());
    let line = segments.iter().map(|s| s.text.as_str()).collect();
    (line, cursor, query.marked_text())
}

/// 词库带声调的照词库标，不带的字按单字表补；输入框里仍是不带调的拼音；开关关着不标。
#[test]
fn words_take_tones_from_the_dictionary() {
    let base = "尴尬\tgan1 ga4\t9000\n好\thao\t8000\n刚\tgang\t5000\n啊\ta\t7000\n你好\tni hao\t9000\n你\tni\t9000\n开发\tkai1 fa1\t9000\n";
    let mut engine = Engine::new(Dictionary::parse(base).unwrap());
    assert_eq!(window_line(&mut engine, "ganga", "尴尬").0, "gan'ga");
    engine.set_show_tones(true);
    assert_eq!(
        window_line(&mut engine, "ganga", "尴尬"),
        ("gān'gà".into(), 6, "gan'ga".into())
    );
    assert_eq!(window_line(&mut engine, "hao", "好").0, "hǎo");
    // 候选没覆盖到的音节照敲的接上；简拼换成候选的完整读音
    assert_eq!(window_line(&mut engine, "nihao", "你").0, "nǐ'hao");
    assert_eq!(window_line(&mut engine, "nh", "你好").0, "nǐ'hǎo");
    assert_eq!(window_line(&mut engine, "kaif", "开发").0, "kāi'fā");
}

/// 整句按词库里的词取声调，词库没有的字退到单字表。
#[test]
fn sentences_take_tones_word_by_word() {
    let base = "我们\two3 men5\t9000\n好\thao\t8000\n我\two\t9000\n们\tmen\t100\n";
    let mut engine = Engine::new(Dictionary::parse(base).unwrap());
    engine.set_show_tones(true);
    assert_eq!(
        window_line(&mut engine, "womenhao", "我们好").0,
        "wǒ'men'hǎo"
    );
}

/// 读音对不上敲的（模糊音、敲错、云端推荐）不换，拼音行仍是用户敲的切分。
#[test]
fn readings_that_do_not_fit_the_input_are_not_shown() {
    let base = "刚\tgang\t5000\n啊\ta\t7000\n刚好\tgang1 hao3\t9000\n";
    let mut engine = Engine::new(Dictionary::parse(base).unwrap());
    engine.set_show_tones(true);
    engine.set_input("ganga");
    let query = engine.query().unwrap();
    let other = Candidate {
        text: "刚好".into(),
        kind: CandidateKind::Cloud,
        syllables: vec!["gang".into(), "hao".into()],
        reading: None,
        translation: None,
        aux_code: None,
    };
    let toned = engine.toned_syllables(&other);
    assert_eq!(
        toned.as_deref(),
        Some(&["gāng".to_owned(), "hǎo".to_owned()][..])
    );
    let (segments, _) = query.window_segments(toned.as_deref());
    let line: String = segments.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(line, query.marked_text());
}
