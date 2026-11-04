//! 面板翻译组：打拼音选词进框、停键后发一次、结果进缓存、回车上屏。

use super::*;
use crate::panel::{PanelGroup, TranslateStatus};
use std::time::{Duration, Instant};

/// 记下请求、等测试塞结果的假翻译提供方。
#[derive(Clone, Default)]
struct Queue {
    submitted: Arc<Mutex<Vec<PredictionRequest>>>,
    replies: Arc<Mutex<Vec<Prediction>>>,
}

impl Predictor for Queue {
    fn policy(&self) -> PredictionPolicy {
        PredictionPolicy {
            automatic: false,
            before: 0,
            after: 0,
            slots: 0,
            max_items: 0,
            sentence: false,
        }
    }

    fn submit(&mut self, request: PredictionRequest) {
        self.submitted.lock().unwrap().push(request);
    }

    fn poll(&mut self) -> Option<Prediction> {
        self.replies.lock().unwrap().pop()
    }
}

impl Queue {
    fn reply_last(&self, text: &str) {
        let sequence = self.submitted.lock().unwrap().last().unwrap().sequence;
        self.replies.lock().unwrap().push(Prediction {
            sequence,
            words: Vec::new(),
            sentence: Some(text.to_owned()),
        });
    }

    fn sent(&self) -> Vec<(String, String)> {
        self.submitted
            .lock()
            .unwrap()
            .iter()
            .map(|r| (r.text.clone(), r.target_language.clone()))
            .collect()
    }
}

fn engine(queue: &Queue) -> Engine {
    let dictionary = Dictionary::parse("你好\tni hao\t100\n世界\tshi jie\t100\n").unwrap();
    Engine::new(dictionary).with_predictor(Box::new(queue.clone()))
}

/// 选第一个候选进框。
fn pick(engine: &mut Engine) {
    let first = engine.query().unwrap().candidates.items[0].clone();
    assert!(engine.commit_to_panel(&first));
}

#[test]
fn typed_words_go_into_the_box_and_are_translated_after_a_pause() {
    let queue = Queue::default();
    let mut engine = engine(&queue);
    engine.set_input("nihao");
    assert!(engine.open_symbol_panel());
    assert_eq!(
        engine.symbol_panel().unwrap().group(),
        PanelGroup::Translate
    );
    pick(&mut engine);
    assert!(engine.composition().is_empty());
    assert_eq!(engine.symbol_panel().unwrap().translate_source(), "你好");

    // 刚选完不发，停够了才发，发过不重发
    let now = Instant::now();
    assert!(!engine.tick_panel_translation(now));
    assert!(queue.sent().is_empty());
    let later = now + Duration::from_secs(1);
    engine.tick_panel_translation(later);
    assert_eq!(queue.sent(), [("你好".to_owned(), "en".to_owned())]);
    engine.tick_panel_translation(later);
    assert_eq!(queue.sent().len(), 1);
    assert!(engine.panel_translation_pending());

    queue.reply_last("Hello");
    // 翻译框的结果不交给壳
    assert_eq!(engine.poll_prediction(), None);
    assert!(engine.tick_panel_translation(later));
    assert_eq!(engine.panel_preedit(), Some("Hello"));
    assert_eq!(
        engine.symbol_panel().unwrap().translate_status(),
        TranslateStatus::Ready
    );

    // 再加两个字、删回去：缓存里有，不重发
    engine.set_input("shijie");
    pick(&mut engine);
    assert_eq!(
        engine.symbol_panel().unwrap().translate_source(),
        "你好世界"
    );
    assert_eq!(engine.panel_preedit(), Some("Hello"));
    assert!(engine.panel_backspace());
    assert!(engine.panel_backspace());
    engine.tick_panel_translation(later + Duration::from_secs(1));
    assert_eq!(queue.sent().len(), 1);

    assert_eq!(engine.take_panel_translation().as_deref(), Some("Hello"));
    assert!(engine.symbol_panel().is_none());
}

#[test]
fn switching_language_translates_at_once_and_is_remembered() {
    let queue = Queue::default();
    let mut engine = engine(&queue);
    engine.set_input("nihao");
    engine.open_symbol_panel();
    pick(&mut engine);
    let later = Instant::now() + Duration::from_secs(1);
    engine.tick_panel_translation(later);
    assert!(engine.symbol_panel_mut().unwrap().switch_section(1));
    engine.tick_panel_translation(later);
    assert_eq!(queue.sent()[1], ("你好".to_owned(), "ja".to_owned()));
    // 回车时日语译文还没到：不上屏
    assert_eq!(engine.take_panel_translation(), None);
    engine.close_symbol_panel();

    // 从菜单打开回到翻译组，语言还是日语
    assert!(engine.open_symbol_panel());
    let panel = engine.symbol_panel().unwrap();
    assert_eq!(panel.group(), PanelGroup::Translate);
    assert_eq!(panel.target_language(), Some(Language::Japanese));
    assert_eq!(panel.translate_source(), "");
}

#[test]
fn foreign_text_is_translated_into_chinese_and_closing_drops_the_request() {
    let queue = Queue::default();
    let mut engine = engine(&queue);
    assert!(engine.open_translate_panel());
    assert!(engine.push_panel_text("good morning"));
    let later = Instant::now() + Duration::from_secs(1);
    engine.tick_panel_translation(later);
    assert_eq!(
        queue.sent()[0],
        ("good morning".to_owned(), "zh".to_owned())
    );
    engine.close_symbol_panel();
    // 关掉后结果回来也不当成别的东西交给壳
    queue.reply_last("早上好");
    assert_eq!(engine.poll_prediction(), None);
}

#[test]
fn without_a_provider_there_is_no_translate_group() {
    let dictionary = Dictionary::parse("你好\tni hao\t100\n").unwrap();
    let mut engine = Engine::new(dictionary);
    assert!(!engine.open_translate_panel());
    engine.set_input("nihao");
    engine.open_symbol_panel();
    let panel = engine.symbol_panel().unwrap();
    assert!(!panel.groups().contains(&PanelGroup::Translate));
    assert_eq!(panel.target_language(), None);
}

#[test]
fn typing_pinyin_over_a_selection_replaces_it() {
    let queue = Queue::default();
    let mut engine = engine(&queue);
    engine.set_input("nihao");
    assert!(engine.open_symbol_panel());
    pick(&mut engine);
    engine.set_input("shijie");
    pick(&mut engine);
    // 拖选「世界」，再打 nihao 选进去
    assert!(engine.place_panel_cursor(2, false));
    assert!(engine.place_panel_cursor(4, true));
    assert_eq!(
        engine.symbol_panel().unwrap().translate_selection(),
        Some(2..4)
    );
    for c in "nihao".chars() {
        engine.push(c);
    }
    let panel = engine.symbol_panel().unwrap();
    assert_eq!(
        (panel.translate_source(), panel.translate_selection()),
        ("你好", None)
    );
    pick(&mut engine);
    assert_eq!(
        engine.symbol_panel().unwrap().translate_source(),
        "你好你好"
    );
    assert!(engine.select_all_panel_text());
    assert!(engine.panel_backspace());
    assert_eq!(engine.symbol_panel().unwrap().translate_source(), "");
}
