use super::*;
use crate::{CandidateLayout, CustomPhrase};
fn phrase(code: &str, position: usize, text: &str) -> CustomPhrase {
    CustomPhrase {
        code: code.into(),
        text: text.into(),
        position,
        enabled: true,
    }
}
#[test]
fn custom_positions_survive_normal_candidates_and_cloud() {
    let mut e = xiaohe();
    e.set_custom_phrases(vec![phrase("ee", 1, "："), phrase("ee", 2, "；")])
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
        }]);
        assert_eq!(layout.candidate(1).unwrap().text, "；");
    }
    let c = e.query().unwrap().candidates.items[1].clone();
    assert_eq!(e.commit(&c), "；");
    assert!(e.composition().text().is_empty());
}
#[test]
fn custom_conflicts_reject_update_and_disabled_rules_stay_disabled() {
    let mut e = engine();
    e.set_custom_phrases(vec![phrase("aa", 1, "，")]).unwrap();
    assert!(
        e.set_custom_phrases(vec![phrase("aa", 1, "，"), phrase("aa", 1, "；")])
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
fn custom_long_text_exact_keys_and_sparse_positions() {
    let mut e = engine();
    let text = format!("{}\n  end ", "长文本".repeat(12000));
    e.set_custom_phrases(vec![phrase("abcdefghij", 3, &text)])
        .unwrap();
    e.set_input("abcdefghij");
    let c = e.query().unwrap().candidates.items[2].clone();
    assert_eq!(e.commit(&c), text);
    e.set_custom_phrases(vec![phrase("ii", 3, "目标")]).unwrap();
    e.set_input("ii");
    let layout = CandidateLayout::new(e.query().unwrap().candidates.items, 9, 2);
    assert_eq!(layout.candidate(2).unwrap().text, "目标");
    for c in layout
        .local()
        .iter()
        .filter(|c| c.kind == CandidateKind::Custom(0))
    {
        assert_eq!(e.commit(c), "");
    }
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
    assert_eq!(e.query().unwrap().candidates.items[1].text, "固定");
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
    assert!(
        e.query()
            .unwrap()
            .candidates
            .items
            .iter()
            .all(|c| c.text != "文本")
    );
}
