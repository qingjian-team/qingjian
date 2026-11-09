//! 拼音行按汉语拼音写 ü：l、n 后写 `ü`（`lve` → `lüe`），j、q、x、y 后写 `u`（`qv` → `qu`）；上屏、学习与输入日志仍是敲法。

use super::*;

fn umlaut_engine() -> Engine {
    let dictionary = "策略\tce lve\t9000\n略\tlve\t5000\n女孩\tnv hai\t8000\n女\tnv\t6000\n提取\tti qu\t7000\n学习\txue xi\t7500\n";
    Engine::new(Dictionary::parse(dictionary).unwrap())
}

fn window_text(query: &Query, toned: Option<&[String]>) -> String {
    let (segments, _) = query.window_segments(toned);
    segments.iter().map(|s| s.text.as_str()).collect()
}

#[test]
fn pinyin_line_follows_pinyin_orthography() {
    let mut engine = umlaut_engine();
    for (input, shown) in [
        ("celve", "ce'lüe"),
        ("celue", "ce'lüe"),
        ("lv", "lü"),
        ("nvhai", "nü'hai"),
        ("tiqv", "ti'qu"),
        ("xvexi", "xue'xi"),
    ] {
        engine.set_input(input);
        let query = engine.query().unwrap();
        assert_eq!(query.marked_text(), shown, "{input}");
        assert_eq!(window_text(&query, None), shown, "{input}");
        assert_eq!(query.marked_cursor(), shown.chars().count(), "{input}");
    }
}

#[test]
fn toned_pinyin_line_keeps_the_umlaut_on_uncovered_syllables() {
    let mut engine = umlaut_engine();
    engine.set_input("celve");
    let query = engine.query().unwrap();
    assert_eq!(window_text(&query, Some(&["cè".to_owned()])), "cè'lüe");

    // 敲 v 与 lue 的也换成带调读音
    engine.set_input("tiqv");
    let query = engine.query().unwrap();
    let toned = ["tí".to_owned(), "qǔ".to_owned()];
    assert_eq!(window_text(&query, Some(&toned)), "tí'qǔ");
    engine.set_input("celue");
    let query = engine.query().unwrap();
    let toned = ["cè".to_owned(), "lüè".to_owned()];
    assert_eq!(window_text(&query, Some(&toned)), "cè'lüè");
}

#[test]
fn rest_after_the_cursor_is_spelled_too() {
    let mut engine = umlaut_engine();
    engine.set_input("nvhailve");
    for _ in 0.."lve".len() {
        engine.move_cursor_left();
    }
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "nü'hai'lüe");
    assert_eq!(query.marked_cursor(), "nü'hai".chars().count());
}

#[test]
fn commit_log_and_raw_text_keep_the_typed_spelling() {
    let entries = Arc::new(Mutex::new(Vec::new()));
    let mut engine = umlaut_engine().with_input_logger(Box::new(MemoryLogger(entries.clone())));
    engine.set_input("celve");
    let first = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(first.syllables, ["ce", "lve"]);
    assert_eq!(engine.commit(&first), "策略");
    {
        let entries = entries.lock().unwrap();
        let InputLogEntry::Commit(commit) = &entries[0] else {
            panic!("expected a commit");
        };
        assert_eq!(commit.pinyin, "ce'lve");
    }

    engine.set_input("nvhai");
    assert_eq!(engine.take_raw(), "nvhai");
}

#[test]
fn nine_key_pinyin_line_is_spelled_but_logged_as_read() {
    let entries = Arc::new(Mutex::new(Vec::new()));
    let mut engine = umlaut_engine().with_input_logger(Box::new(MemoryLogger(entries.clone())));
    engine.set_nine_key(true);
    // 5 = jkl、8 = tuv：这本词库里 58 只读得通 lv（略 lve 的前缀）
    engine.set_input("58");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "略");
    assert_eq!(query.marked_text(), "lü");
    let first = query.candidates.items[0].clone();
    engine.commit(&first);
    let entries = entries.lock().unwrap();
    let InputLogEntry::Commit(commit) = &entries[0] else {
        panic!("expected a commit");
    };
    assert_eq!(commit.pinyin, "lv");
}

#[test]
fn decoded_shuangpin_is_spelled_with_the_umlaut() {
    let mut engine = umlaut_engine();
    engine.set_shuangpin(Some(Scheme::Xiaohe));
    engine.set_input("nvhd");
    assert_eq!(engine.query().unwrap().marked_text(), "nü'hai");
}
