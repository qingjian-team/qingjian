//! 按词跳快捷键：只在组句时拿走；首选是整句时定住第一个词，跳不动也吃掉（组句中不粘贴）。
use super::tests::{compose, key};
use crate::dispatch::{Router, RouterConfig};
use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_platform::KeyCombo;
use qingjian_platform::protocol::{
    ClientMessage, KeyModifiers, KeyOutcome, PROTOCOL_VERSION, SessionId,
};

const V: u32 = 0x56;

fn ctrl() -> KeyModifiers {
    KeyModifiers {
        ctrl: true,
        ..Default::default()
    }
}

fn router() -> Router {
    let dictionary = concat!(
        "交易\tjiao yi\t5000\n中心\tzhong xin\t5000\n忠心\tzhong xin\t3000\n",
        "你好\tni hao\t9000\n",
    );
    let mut router = Router::new(
        Engine::new(Dictionary::parse(dictionary).unwrap()),
        RouterConfig::default(),
    );
    router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    router
}

#[test]
fn ctrl_v_holds_the_first_word_while_composing() {
    let mut router = router();
    assert_eq!(router.config.next_word, Some(KeyCombo::NEXT_WORD_DEFAULT));
    // 没在组句：交还应用（粘贴）
    assert_eq!(key(&mut router, V, None, ctrl()).0, KeyOutcome::Passthrough);

    compose(&mut router, "jiaoyizhongxin", KeyModifiers::default());
    let (outcome, commit, frame) = key(&mut router, V, None, ctrl());
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(commit.is_none());
    assert_eq!(router.engine.held_text(), "交易");
    assert_eq!(router.engine.composition().text(), "zhongxin");
    assert!(frame.candidates.items.iter().any(|c| c.text == "忠心"));
}

#[test]
fn ctrl_v_on_a_single_word_is_swallowed() {
    let mut router = router();
    compose(&mut router, "nihao", KeyModifiers::default());
    let (outcome, commit, _) = key(&mut router, V, None, ctrl());
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(commit.is_none());
    assert_eq!(router.engine.composition().text(), "nihao");
    assert_eq!(router.engine.held_text(), "");
}
