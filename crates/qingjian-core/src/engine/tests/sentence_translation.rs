//! 自动译文只跟随当前学习语言，保留中文排序并丢弃过期结果。
use super::*;

struct SinglePredictor {
    replies: Vec<Prediction>,
}
impl Predictor for SinglePredictor {
    fn policy(&self) -> PredictionPolicy {
        PredictionPolicy {
            sentence_translation: true,
            ..PredictionPolicy::default()
        }
    }
    fn submit(&mut self, request: PredictionRequest) {
        self.replies.push(Prediction {
            sequence: request.sequence,
            translation: request
                .translation_source
                .map(|source| SentenceTranslation {
                    source,
                    language: request.target_language.parse().unwrap(),
                    text: "Develop".into(),
                }),
            ..Prediction::default()
        });
    }
    fn poll(&mut self) -> Option<Prediction> {
        if self.replies.is_empty() {
            None
        } else {
            Some(self.replies.remove(0))
        }
    }
}
struct LearningEnglish;
impl Translator for LearningEnglish {
    fn language(&self) -> Language {
        Language::English
    }
    fn translate(&self, _: &str) -> Option<Translation> {
        None
    }
}
fn translated_engine() -> Engine {
    let mut e = engine().with_predictor(Box::new(SinglePredictor {
        replies: Vec::new(),
    }));
    e.set_translator(Box::new(LearningEnglish));
    e
}
#[test]
fn automatic_translation_annotates_one_chinese_candidate_without_adding_rows() {
    let mut e = translated_engine();
    e.set_input("kaifa");
    let before = e.query().unwrap().candidates.items;
    e.request_prediction(None, &before);
    let prediction = e.poll_prediction().unwrap();
    assert_eq!(
        prediction.translation.as_ref().unwrap().language,
        Language::English
    );
    let after = e.query().unwrap().candidates.items;
    assert_eq!(
        before.iter().map(|c| &c.text).collect::<Vec<_>>(),
        after.iter().map(|c| &c.text).collect::<Vec<_>>()
    );
    let mut candidates = CandidateList { items: after };
    e.annotate(&mut candidates);
    assert_eq!(candidates.items[0].text, "开发");
    assert_eq!(
        candidates.items[0].translation.as_ref().unwrap().senses()[0].text,
        "Develop"
    );
}
#[test]
fn stale_private_and_cancelled_responses_do_not_annotate() {
    let mut e = translated_engine();
    e.set_input("kaifa");
    let first = e.query().unwrap().candidates.items;
    e.request_prediction(None, &first);
    e.set_input("kaifazhe");
    let newer = e.query().unwrap().candidates.items;
    e.request_prediction(None, &newer);
    assert_eq!(
        e.poll_prediction().unwrap().translation.unwrap().source,
        "开发者"
    );
    e.set_private(true);
    let mut list = CandidateList {
        items: e.query().unwrap().candidates.items,
    };
    e.annotate(&mut list);
    assert!(list.items.iter().all(|c| c.translation.is_none()));
    e.set_private(false);
    e.cancel_prediction();
    e.annotate(&mut list);
    assert!(list.items.iter().all(|c| c.translation.is_none()));
}
#[test]
fn explicit_selection_target_preserves_whitespace_and_does_not_touch_composition() {
    let mut e = translated_engine();
    e.set_input("kaifa");
    assert!(
        e.request_translation_to("  今天讨论项目。\n", Language::Japanese)
            .is_some()
    );
    assert_eq!(e.composition().text(), "kaifa");
    e.set_private(true);
    assert!(
        e.request_translation_to("秘密", Language::English)
            .is_none()
    );
}
