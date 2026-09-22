//! 本地整句模型：停顿后重排换首选、动过高亮不换、模型在组句中途接上也补这一轮。
use std::time::{Duration, Instant};

use qingjian_core::Engine;
use qingjian_core::sentence::SentenceScorer;
use qingjian_dictionary::Dictionary;
use qingjian_linux_server::{Router, RouterConfig};
use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyModifiers, PROTOCOL_VERSION, ServerMessage, SessionId,
};

const SESSION: SessionId = SessionId(1);

/// 假打分器：偏爱某个文本，其余都给低分（与 Core / Windows Server 的重打分测试同款）。
struct Prefers(&'static str);

impl SentenceScorer for Prefers {
    fn score(&self, _context: &str, texts: &[&str]) -> Vec<f64> {
        texts
            .iter()
            .map(|t| if *t == self.0 { -1.0 } else { -20.0 })
            .collect()
    }
}

/// k 优路径按末词分状态，几条路径要在末词上不同才都留下来：ni + ta → 你他 / 你她 / 你它。
fn engine() -> Engine {
    Engine::new(Dictionary::parse("你\tni\t100\n他\tta\t100\n她\tta\t90\n它\tta\t80\n").unwrap())
}

fn router_with(engine: Engine) -> Router {
    let mut router = Router::new(engine, RouterConfig::default());
    router.handle(ClientMessage::OpenSession {
        session: SESSION,
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    router.handle(ClientMessage::Privacy {
        session: SESSION,
        private: false,
    });
    router
}

/// 接了假模型的 Router：本地整句模型在 Server 里是异步接法，按键先按词级出候选，停顿后 tick 才换。
fn router_with_scorer(preferred: &'static str) -> Router {
    let mut engine = engine();
    engine.set_async_sentence_scorer(Some(Box::new(Prefers(preferred))));
    router_with(engine)
}

fn press(router: &mut Router, event: KeyEvent) -> Frame {
    match router.handle(ClientMessage::Key {
        session: SESSION,
        event,
    }) {
        Some(ServerMessage::KeyResult { frame, .. }) => frame,
        other => panic!("expected KeyResult, got {other:?}"),
    }
}

fn type_letters(router: &mut Router, text: &str) -> Frame {
    let mut last = Frame::default();
    for c in text.chars() {
        last = press(
            router,
            KeyEvent::new(
                c.to_ascii_uppercase() as u32,
                Some(c),
                KeyModifiers::default(),
            ),
        );
    }
    last
}

fn first(frame: &Frame) -> Option<&str> {
    frame.candidates.items.first().map(|c| c.text.as_str())
}

fn poll(router: &mut Router) -> Frame {
    match router.handle(ClientMessage::Poll { session: SESSION }) {
        Some(ServerMessage::Update { frame, .. }) => frame,
        other => panic!("expected Update, got {other:?}"),
    }
}

/// 一直 tick 到首选变成 `text` 或等满 `timeout`；返回最后一帧。
fn tick_until_first(router: &mut Router, text: &str, timeout: Duration) -> Frame {
    let started = Instant::now();
    loop {
        std::thread::sleep(router.next_tick().min(Duration::from_millis(20)));
        router.tick();
        let frame = poll(router);
        if first(&frame) == Some(text) || started.elapsed() > timeout {
            return frame;
        }
    }
}

#[test]
fn local_model_rescoring_reorders_sentence_after_pause() {
    let mut router = router_with_scorer("你它");
    let frame = type_letters(&mut router, "nita");
    // 按键时只按词级模型：他 的词频高，首选是「你他」
    assert_eq!(first(&frame), Some("你他"));
    // 在等防抖，主循环该在 80 ms 内醒来
    assert!(router.next_tick() <= Duration::from_millis(80));

    let frame = tick_until_first(&mut router, "你它", Duration::from_secs(3));
    assert_eq!(
        first(&frame),
        Some("你它"),
        "停顿后模型偏爱的整句应换到首位，实际：{:?}",
        frame.candidates.items
    );
    // 换完不再等；空闲节拍回到落盘学习的一秒
    assert_eq!(router.next_tick(), Duration::from_secs(1));
}

#[test]
fn local_model_does_not_touch_a_navigated_page() {
    let mut router = router_with_scorer("你它");
    type_letters(&mut router, "nita");
    // 用户动过高亮（下方向键）：模型的结果只留在缓存里，不换正在看的这页
    let frame = press(
        &mut router,
        KeyEvent::new(0x28, None, KeyModifiers::default()),
    );
    assert_eq!(first(&frame), Some("你他"));
    // 防抖 80 ms + 假模型立即回分，300 ms 足够等到结果；首选仍是原来的
    let frame = tick_until_first(&mut router, "你它", Duration::from_millis(300));
    assert_eq!(first(&frame), Some("你他"));
}

#[test]
fn model_attached_during_composition_rescores_the_current_round() {
    // 模型还在后台加载时用户已经在组句：这一轮的查询从没见过打分器
    let mut router = router_with(engine());
    let frame = type_letters(&mut router, "nita");
    assert_eq!(first(&frame), Some("你他"));
    assert_eq!(router.next_tick(), Duration::from_secs(1));

    // 加载完成接上：不用再敲键，这一轮也补一次重排
    router.attach_sentence_scorer(Box::new(Prefers("你它")));
    assert!(router.next_tick() <= Duration::from_millis(80));
    let frame = tick_until_first(&mut router, "你它", Duration::from_secs(3));
    assert_eq!(first(&frame), Some("你它"));
}
