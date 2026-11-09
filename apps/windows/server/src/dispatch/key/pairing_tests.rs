//! 成对符号（#928）：中文括号补右半边、光标退回；紧接着的右半边让 DLL 核对后跳过；
//! 英文模式缺省不成对，开了才补半角括号。
use crate::dispatch::{Router, RouterConfig};
use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_platform::protocol::{
    CaretEdit, ClientMessage, KeyEvent, KeyModifiers, KeyOutcome, PROTOCOL_VERSION, ServerMessage,
    SessionId,
};

/// 回 (吃不吃, 上屏文本, 光标编辑)。
fn press(router: &mut Router, event: KeyEvent) -> (KeyOutcome, Option<String>, Option<CaretEdit>) {
    match router.handle(ClientMessage::Key {
        session: SessionId(1),
        event,
    }) {
        Some(ServerMessage::KeyResult {
            outcome,
            commit,
            caret,
            ..
        }) => (outcome, commit, caret),
        other => panic!("expected key result, got {other:?}"),
    }
}

fn typed(c: char, modifiers: KeyModifiers) -> KeyEvent {
    // 虚拟键码对标点无所谓，按字符走；避开小键盘段
    KeyEvent::new(0xBA, Some(c), modifiers)
}

fn router() -> Router {
    let mut router = Router::new(
        Engine::new(Dictionary::parse("你\tni\t9000\n").unwrap()),
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
fn chinese_brackets_pair_and_the_closer_skips() {
    let mut router = router();
    let chinese = KeyModifiers::default();
    assert_eq!(
        press(&mut router, typed('(', chinese)),
        (
            KeyOutcome::Consumed,
            Some("（）".to_owned()),
            Some(CaretEdit::Back(1))
        )
    );
    // 中间打了字之后敲右括号：交给 DLL 核对后跳过
    press(&mut router, typed('1', chinese));
    assert_eq!(
        press(&mut router, typed(')', chinese)),
        (
            KeyOutcome::Consumed,
            Some("）".to_owned()),
            Some(CaretEdit::Skip('）'))
        )
    );
}

#[test]
fn english_mode_pairs_half_width_only_when_switched_on() {
    let mut router = router();
    let english = KeyModifiers {
        english_mode: true,
        ..KeyModifiers::default()
    };
    assert_eq!(
        press(&mut router, typed('(', english)),
        (KeyOutcome::Passthrough, None, None)
    );
    router.engine.set_pair_punctuation(true, true);
    assert_eq!(
        press(&mut router, typed('(', english)),
        (
            KeyOutcome::Consumed,
            Some("()".to_owned()),
            Some(CaretEdit::Back(1))
        )
    );
}
