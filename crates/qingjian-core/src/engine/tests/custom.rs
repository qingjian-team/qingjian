use super::*;
use crate::{CandidateLayout, CustomPhrase};

fn phrase(code: &str, weight: u32, text: &str) -> CustomPhrase {
    CustomPhrase {
        weight,
        ..CustomPhrase::new(code, text)
    }
}

#[test]
fn custom_positions_survive_normal_candidates_and_cloud() {
    let mut e = xiaohe();
    e.set_custom_phrases(vec![phrase("ee", 10, "："), phrase("ee", 10, "；")])
        .unwrap();
    e.set_input("ee");
    for _ in 0..3 {
        let q = e.query().unwrap();
        assert_eq!(q.candidates.items[0].text, "：");
        assert_eq!(q.candidates.items[1].text, "；");
        let mut layout = CandidateLayout::new(q.candidates.items, 2, 2);
        layout.set_cloud(vec![Candidate {
            text: "云".into(),
            kind: CandidateKind::Cloud,
            syllables: vec![],
            reading: None,
            translation: None,
            aux_code: None,
        }]);
        assert_eq!(layout.candidate(1).unwrap().text, "；");
    }
    let c = e.query().unwrap().candidates.items[1].clone();
    assert_eq!(e.commit(&c), "；");
    assert!(e.composition().text().is_empty());
}

#[test]
fn heavier_phrases_go_first_and_ties_keep_their_order() {
    let mut e = engine();
    e.set_custom_phrases(vec![
        phrase("dz", 5, "乙"),
        phrase("dz", 20, "甲"),
        phrase("dz", 5, "丙"),
    ])
    .unwrap();
    e.set_input("dz");
    let items = e.query().unwrap().candidates.items;
    let texts: Vec<_> = items.iter().take(3).map(|c| c.text.as_str()).collect();
    assert_eq!(texts, ["甲", "乙", "丙"]);
    assert_eq!(items[2].kind, CandidateKind::Custom(3));
    assert!(
        items[3..]
            .iter()
            .all(|c| !matches!(c.kind, CandidateKind::Custom(_)))
    );
}

#[test]
fn english_mode_uses_custom_phrases_too() {
    // #763：`dont` → don't 在英文模式（Caps Lock 亮着）下也该生效，且排第一
    let words =
        WordList::parse("dont\tdont\t5000\ndone\tdone\t4000\ndonate\tdonate\t3000\n\t").unwrap();
    let mut e = engine().with_english(words);
    e.set_english_mode(true);
    e.set_custom_phrases(vec![phrase("dont", 10, "don't")])
        .unwrap();
    e.set_input("dont");
    let items = e.query().unwrap().candidates.items;
    assert_eq!(items[0].text, "don't");
    assert_eq!(items[0].kind, CandidateKind::Custom(1));
    // 空格上屏首选就是短语
    assert_eq!(e.commit(&items[0]), "don't");
    // 别的输入不受影响
    e.set_input("don");
    let items = e.query().unwrap().candidates.items;
    assert!(
        items.iter().all(|c| c.kind != CandidateKind::Custom(1)),
        "{items:?}"
    );
}

#[test]
fn a_phrase_hides_the_same_word_among_normal_candidates() {
    let mut e = engine();
    e.set_input("kaifa");
    let word = e.query().unwrap().candidates.items[0].text.clone();
    e.set_custom_phrases(vec![phrase("kaifa", 10, &word)])
        .unwrap();
    e.set_input("kaifa");
    let items = e.query().unwrap().candidates.items;
    assert_eq!(items[0].kind, CandidateKind::Custom(1));
    assert_eq!(items.iter().filter(|c| c.text == word).count(), 1);
}

#[test]
fn custom_conflicts_reject_update_and_disabled_rules_stay_disabled() {
    let mut e = engine();
    e.set_custom_phrases(vec![phrase("aa", 1, "，")]).unwrap();
    assert!(
        e.set_custom_phrases(vec![phrase("aa", 1, "，"), phrase("aa", 3, "，")])
            .is_err()
    );
    e.set_input("aa");
    assert_eq!(e.query().unwrap().candidates.items[0].text, "，");
    let mut disabled = phrase("prompt", 1, "不启用");
    disabled.enabled = false;
    e.set_custom_phrases(vec![disabled]).unwrap();
    e.set_input("prompt");
    assert!(
        e.query()
            .unwrap()
            .candidates
            .items
            .iter()
            .all(|c| c.text != "不启用")
    );
}

#[test]
fn custom_long_text_and_exact_keys() {
    let mut e = engine();
    let text = format!("{}\n  end ", "长文本".repeat(12000));
    e.set_custom_phrases(vec![phrase("abcdefghij", 3, &text)])
        .unwrap();
    e.set_input("abcdefghij");
    let query = e.query().unwrap();
    let layout = CandidateLayout::new(query.candidates.items, 9, 2);
    let c = layout.candidate(0).unwrap().clone();
    assert_eq!(e.commit(&c), text);
    e.set_custom_phrases(vec![phrase("ii", 3, "目标")]).unwrap();
    e.set_input("ii");
    let layout = CandidateLayout::new(e.query().unwrap().candidates.items, 9, 2);
    assert_eq!(layout.candidate(0).unwrap().text, "目标");
    assert!(layout.local().iter().all(|c| !c.text.is_empty()));
    assert!(
        e.last_query
            .borrow()
            .as_ref()
            .unwrap()
            .candidates
            .iter()
            .all(|s| !s.is_empty())
    );
    assert_eq!(e.composition().text(), "ii");
}

#[test]
fn punctuation_mode_does_not_change_custom_text() {
    let mut e = engine();
    e.set_full_width_punctuation(false);
    for c in [',', ';', ':', 'a', '1'] {
        assert_eq!(e.punctuate(c), None);
    }
    e.set_custom_phrases(vec![phrase("bb", 1, "；")]).unwrap();
    e.set_input("bb");
    let c = e.query().unwrap().candidates.items[0].clone();
    assert_eq!(e.commit(&c), "；");
    e.set_full_width_punctuation(true);
    assert_eq!(e.punctuate(';'), Some("；"));
}

#[test]
fn custom_exact_codes_override_mode_prefixes_but_not_longer_input() {
    let mut e = engine();
    e.set_custom_phrases(vec![phrase("vv", 2, "固定"), phrase("uu", 1, "文本")])
        .unwrap();
    e.set_input("vv");
    assert!(!e.expression_mode());
    let layout = CandidateLayout::new(e.query().unwrap().candidates.items, 9, 2);
    assert_eq!(layout.candidate(0).unwrap().text, "固定");
    e.set_input("vvv");
    assert!(e.expression_mode());
    assert!(
        e.query()
            .unwrap()
            .candidates
            .items
            .iter()
            .all(|c| c.text != "固定")
    );
    e.set_input("uu");
    assert!(!e.question_mode());
    e.set_english_mode(true);
    // 英文模式也照出这个短语（#763），且排在英文候选前
    assert_eq!(e.query().unwrap().candidates.items[0].text, "文本");
}

#[test]
fn custom_only_query_preserves_raw_preedit_and_logs_custom_source() {
    let mut e = engine();
    e.set_custom_phrases(vec![phrase("ii", 3, "目标")]).unwrap();
    e.set_input("ii");
    let query = e.query().unwrap();
    assert_eq!(query.marked_text(), "ii");
    assert_eq!(query.marked_cursor(), 2);
    assert_eq!(query.candidates.items.len(), 1);
    assert_eq!(query.candidates.items[0].kind, CandidateKind::Custom(1));
    assert_eq!(
        InputSource::from(query.candidates.items[0].kind),
        InputSource::Custom
    );
}

#[test]
fn custom_preview_handles_unicode_controls_and_limits() {
    assert_eq!(CustomPhrase::preview("甲\r\n乙\t🙂尾", 5), "甲↵乙⇥🙂…");
    assert_eq!(CustomPhrase::preview("甲\n乙", 3), "甲↵乙");
    assert_eq!(CustomPhrase::preview("甲", 0), "…");
    assert_eq!(CustomPhrase::preview("", 0), "");
}
