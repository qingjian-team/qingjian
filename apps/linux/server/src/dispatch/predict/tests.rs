//! 云联想在 Linux 壳里的接线：组句发一次请求、结果并入第一页末尾、没结果不画占位、取消与隐私门槛。
//! 全程用假联想器，不发网络请求。

use crate::dispatch::{Router, RouterConfig};
use crate::protocol::{Capabilities, LinuxEvent, LinuxRequest};
use qingjian_core::{
    CandidateKind, CloudWord, Engine, NoPredictor, Prediction, PredictionPolicy, PredictionRequest,
    Predictor,
};
use qingjian_dictionary::Dictionary;
use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyModifiers, PROTOCOL_VERSION, ServerMessage, SessionId,
};

/// 记一次请求、下一次 `poll` 才给结果的假联想器；序号跟着最新请求，和真预测器一样。
#[derive(Default)]
struct Fake {
    /// 等在外面的结果序号；`None` 表示没有待回的结果。
    pending: Option<u64>,

    /// 要回的云端词。
    words: Vec<CloudWord>,

    /// 要回的整句补全。
    sentence: Option<String>,

    /// 第一页给云端留几格（`[predict] slots`）。
    slots: usize,
}

impl Predictor for Fake {
    fn policy(&self) -> PredictionPolicy {
        PredictionPolicy {
            before: 0,
            after: 0,
            slots: self.slots,
            max_items: self.slots + 2,
            sentence: true,
        }
    }

    fn submit(&mut self, request: PredictionRequest) {
        self.pending = Some(request.sequence);
    }

    fn poll(&mut self) -> Option<Prediction> {
        let sequence = self.pending.take()?;
        Some(Prediction {
            sequence,
            words: std::mem::take(&mut self.words),
            sentence: self.sentence.take(),
        })
    }
}

fn cloud_word(text: &str, syllables: &[&str]) -> CloudWord {
    CloudWord {
        text: text.to_owned(),
        syllables: syllables.iter().map(|s| (*s).to_owned()).collect(),
        reading: None,
    }
}

/// 一页 5 格、云端留 `slots` 格，一被问就回「拟 / 你好」的 Router。
fn router(slots: usize) -> Router {
    let mut engine = Engine::new(
        Dictionary::parse(
            "你\tni\t100\n好\tni hao\t90\n世\tni she\t80\n界\tni jie\t70\n上\tni shang\t60\n",
        )
        .unwrap(),
    );
    engine.set_predictor(Box::new(Fake {
        words: vec![cloud_word("拟", &["ni"])],
        sentence: Some("你好".to_owned()),
        slots,
        ..Default::default()
    }));
    let mut router = Router::new(
        engine,
        RouterConfig {
            page_size: 5,
            cloud_slots: slots,
            ..Default::default()
        },
    );
    router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    // 会话默认按私密处理，插件一开就报能力；不报的话什么都不会发出去。
    report(&mut router, Capabilities::default());
    router
}

/// 上报输入框能力。
fn report(router: &mut Router, caps: Capabilities) {
    router.linux_event(LinuxRequest {
        session: SessionId(1),
        event: LinuxEvent::Capabilities(caps),
    });
}

fn key(router: &mut Router, code: u32, character: Option<char>) -> (Option<String>, Frame) {
    match router
        .handle(ClientMessage::Key {
            session: SessionId(1),
            event: KeyEvent::new(code, character, KeyModifiers::default()),
        })
        .unwrap()
    {
        ServerMessage::KeyResult { commit, frame, .. } => (commit, frame),
        _ => panic!("key result"),
    }
}

/// 敲完一串字母，返回最后一次按键的帧（还没 tick，云端结果不可能在里面）。
fn compose(router: &mut Router, text: &str) -> Frame {
    let mut frame = Frame::default();
    for c in text.chars() {
        frame = key(router, u32::from(c), Some(c)).1;
    }
    frame
}

/// 插件组句期间每 80 毫秒问一次，顺带 tick：回的帧就是最新状态。
fn polled(router: &mut Router) -> Frame {
    match router
        .handle(ClientMessage::Poll {
            session: SessionId(1),
        })
        .unwrap()
    {
        ServerMessage::Update { frame, .. } => frame,
        _ => panic!("update"),
    }
}

fn has_cloud(frame: &Frame) -> bool {
    frame
        .candidates
        .items
        .iter()
        .any(|candidate| candidate.kind == CandidateKind::Cloud)
}

#[test]
fn cloud_word_lands_at_the_tail_of_the_first_page_with_a_sentence() {
    let mut router = router(2);
    let before = compose(&mut router, "ni");
    assert!(router.predict.polling(), "组句应当发了一次联想请求");
    assert!(
        !has_cloud(&before) && before.sentence.is_none(),
        "结果没回来之前不预留、不画占位"
    );

    let after = polled(&mut router);
    assert_eq!(after.sentence.as_deref(), Some("你好"));
    assert_eq!(
        after.candidates.items.last().map(|c| c.kind),
        Some(CandidateKind::Cloud),
        "云端词补在第一页末尾，本地候选不动"
    );
    assert_eq!(after.candidates.items.first().unwrap().text, "你");
    assert!(!router.predict.polling(), "收到结果就不必再等下一次 tick");
}

#[test]
fn slots_zero_keeps_only_the_sentence() {
    let mut router = router(0);
    compose(&mut router, "ni");
    assert!(router.predict.polling(), "只要整句也照样要问一次");
    let frame = polled(&mut router);
    assert_eq!(frame.sentence.as_deref(), Some("你好"));
    assert!(!has_cloud(&frame), "[predict] slots = 0 时不要云端词");
}

#[test]
fn tab_accepts_the_cloud_sentence() {
    let mut router = router(2);
    compose(&mut router, "ni");
    polled(&mut router);
    let (commit, _) = key(&mut router, 9, None);
    assert_eq!(
        commit.as_deref(),
        Some("你好"),
        "Tab 接受整句补全并整段上屏"
    );
    assert_eq!(router.sentence, None, "接受过一次就不再挂着");
}

#[test]
fn finishing_the_composition_stops_waiting() {
    let mut router = router(2);
    compose(&mut router, "ni");
    assert!(router.predict.polling());
    key(&mut router, 0x1b, None);
    assert!(
        !router.predict.polling(),
        "组句结束了，迟到的结果不该再画出来"
    );
}

#[test]
fn sensitive_input_asks_nothing() {
    let mut router = router(2);
    key(&mut router, u32::from('n'), Some('n'));
    report(
        &mut router,
        Capabilities {
            sensitive: true,
            password: false,
            disabled: false,
        },
    );
    compose(&mut router, "i");
    assert!(
        !router.predict.polling(),
        "敏感输入不发云联想请求，光标附近的文字一个字也不出去"
    );
    assert!(!has_cloud(&polled(&mut router)));
}

#[test]
fn a_router_without_cloud_never_starts_the_wait() {
    let mut engine = Engine::new(Dictionary::parse("你\tni\t100\n").unwrap());
    engine.set_predictor(Box::new(NoPredictor));
    let mut router = Router::new(
        engine,
        RouterConfig {
            page_size: 5,
            cloud_slots: 2,
            ..Default::default()
        },
    );
    router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    report(&mut router, Capabilities::default());
    compose(&mut router, "ni");
    assert!(!router.predict.polling(), "[predict] 关着就不该等结果");
    assert!(!has_cloud(&polled(&mut router)));
}
