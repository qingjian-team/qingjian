//! emoji。

use super::*;

#[test]
fn emoji_limit_preserves_words_and_can_be_disabled_and_restored() {
    let table = EmojiTable::parse("开发\t👨‍💻 🛠️ 🧑‍💻\n开\t🔓\n").unwrap();
    let mut engine = engine().with_emoji(table);
    engine.set_input("kaifazhe");
    let original = engine.query().unwrap().candidates.items;
    let words: Vec<_> = original
        .iter()
        .filter(|c| c.kind != CandidateKind::Emoji)
        .map(|c| c.text.clone())
        .collect();
    for limit in [0, 1, 2, 3, usize::MAX, 0, 3] {
        engine.set_emoji_limit(limit);
        let items = engine.query().unwrap().candidates.items;
        assert_eq!(
            items
                .iter()
                .filter(|c| c.kind == CandidateKind::Emoji)
                .count(),
            limit.min(3)
        );
        assert_eq!(
            items
                .iter()
                .filter(|c| c.kind != CandidateKind::Emoji)
                .map(|c| c.text.clone())
                .collect::<Vec<_>>(),
            words
        );
        assert_eq!(engine.composition().text(), "kaifazhe");
    }
}

#[test]
fn emoji_limit_applies_in_english_mode_and_keeps_commit_behavior() {
    let words = WordList::parse("smile\nsmiled\n").unwrap();
    let table = EmojiTable::parse("smile\t😀 😄\n").unwrap();
    let mut engine = engine().with_english(words).with_emoji(table);
    engine.set_english_mode(true);
    engine.set_input("smile");
    engine.set_emoji_limit(0);
    assert!(
        engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .all(|c| c.kind != CandidateKind::Emoji)
    );
    engine.set_emoji_limit(1);
    let items = engine.query().unwrap().candidates.items;
    let emojis: Vec<_> = items
        .iter()
        .filter(|c| c.kind == CandidateKind::Emoji)
        .collect();
    assert_eq!(emojis.len(), 1);
    assert_eq!(engine.commit(emojis[0]), "😀");
    assert!(engine.composition().is_empty());
}

#[test]
fn english_words_bring_their_emoji_and_the_emoji_consumes_the_whole_input() {
    let words = WordList::parse("smile\nsmiled\n").unwrap();
    let table = EmojiTable::parse("smile\t😀 😄\n笑\t😄\n").unwrap();
    let mut engine = engine().with_english(words).with_emoji(table);
    engine.set_input("smile");
    let items = engine.query().unwrap().candidates.items;
    let position = items
        .iter()
        .position(|c| c.kind == CandidateKind::English && c.text == "smile")
        .unwrap();
    let emoji = items[position + 1].clone();
    assert_eq!(emoji.kind, CandidateKind::Emoji);
    assert_eq!(emoji.text, "😀");
    assert_eq!(emoji.reading.as_deref(), Some("smile"));
    assert_eq!(engine.commit(&emoji), "😀");
    assert!(engine.composition().is_empty());
}

#[test]
fn emoji_follow_their_word_and_consume_its_syllables() {
    let table = EmojiTable::parse("开发\t👨‍💻 🛠️ 🧑‍💻\n开\t🔓\n").unwrap();
    let mut engine = engine().with_emoji(table);
    engine.set_input("kaifazhe");
    let query = engine.query().unwrap();
    let items = &query.candidates.items;
    let kaifa = items.iter().position(|c| c.text == "开发").unwrap();
    assert_eq!(items[kaifa + 1].text, "👨‍💻");
    assert_eq!(items[kaifa + 1].kind, CandidateKind::Emoji);
    assert_eq!(items[kaifa + 1].reading.as_deref(), Some("开发"));
    assert_eq!(items[kaifa + 2].text, "🛠️");
    // 每个词最多两个，一次最多三个
    assert_eq!(
        items
            .iter()
            .filter(|c| c.kind == CandidateKind::Emoji)
            .count(),
        3
    );
    let emoji = items[kaifa + 1].clone();
    assert_eq!(engine.commit(&emoji), "👨‍💻");
    assert_eq!(engine.composition().text(), "zhe");
}
