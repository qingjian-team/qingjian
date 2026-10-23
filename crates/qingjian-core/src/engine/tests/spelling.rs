//! 拼音行按候选的读音切分：`ganga` 高亮 尴尬 时 `gan'ga`，高亮 刚啊 时 `gang'a`。

use super::*;

#[test]
fn pinyin_line_follows_the_candidate() {
    let base = "尴尬\tgan1 ga4\t9000\n刚\tgang\t5000\n啊\ta\t7000\n干\tgan\t3000\n";
    let mut engine = Engine::new(Dictionary::parse(base).unwrap());
    engine.set_input("ganga");
    let mut query = engine.query().unwrap();
    let find = |query: &Query, text: &str| {
        query
            .candidates
            .items
            .iter()
            .find(|c| c.text == text)
            .cloned()
    };
    // 首选是哪个，拼音行就按哪个切
    let expected = match query.candidates.items[0].text.as_str() {
        "尴尬" => "gan'ga",
        _ => "gang'a",
    };
    assert_eq!(query.marked_text(), expected);
    let awkward = find(&query, "尴尬").unwrap();
    let gang = find(&query, "刚").unwrap();
    query.follow(Some(&gang));
    assert_eq!(query.marked_text(), "gang'a");
    assert_eq!(query.marked_cursor(), 6);
    query.follow(Some(&awkward));
    assert_eq!(query.marked_text(), "gan'ga");
    query.follow(None);
    assert_eq!(query.marked_text(), "gang'a");
}
