//! 组句与云联想的光标前文。

use crate::dispatch::{Router, RouterConfig};
use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_platform::protocol::PROTOCOL_VERSION;
use qingjian_platform::protocol::{ClientMessage, KeyEvent, KeyModifiers, SessionId};

fn router() -> Router {
    let mut router = Router::new(
        Engine::new(Dictionary::parse("你\tni\t100\n").unwrap()),
        RouterConfig::default(),
    );
    router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    router
}

fn compose(router: &mut Router, text: &str) {
    for c in text.chars() {
        router
            .handle(ClientMessage::Key {
                session: SessionId(1),
                event: KeyEvent::new(c as u32, Some(c), KeyModifiers::default()),
            })
            .unwrap();
    }
}

/// 回归 #283：DLL 报来的光标前文要成为云联想的 `before` 上下文；
/// 没前文时不带上下文，空文本不算上下文，组句结束即作废。
#[test]
fn surrounding_context_flows_to_prediction_and_clears_on_reset() {
    let mut router = router();
    compose(&mut router, "ni");
    assert!(router.surrounding_for_prediction().is_none());

    router.handle(ClientMessage::Surrounding {
        session: SessionId(1),
        text: "这个文件权限有问题，".to_owned(),
    });
    let surrounding = router.surrounding_for_prediction().expect("应带上前文");
    assert_eq!(surrounding.before, "这个文件权限有问题，");
    assert_eq!(surrounding.after, "");

    // 空文本不算上下文（Engine 只靠拼音猜，不猜错的）
    router.handle(ClientMessage::Surrounding {
        session: SessionId(1),
        text: String::new(),
    });
    assert!(router.surrounding_for_prediction().is_none());

    // 组句结束：前文作废，等下一段组句 DLL 再送
    router.handle(ClientMessage::Surrounding {
        session: SessionId(1),
        text: "别的内容".to_owned(),
    });
    router.stop_rescoring();
    assert!(router.surrounding_for_prediction().is_none());
}

/// 不是聚焦会话的前文不收，防止换应用后把 A 应用的文字带进 B 的联想。
#[test]
fn surrounding_from_unfocused_session_is_dropped() {
    let mut router = router();
    compose(&mut router, "ni");
    router.handle(ClientMessage::Surrounding {
        session: SessionId(99),
        text: "别的应用的前文".to_owned(),
    });
    assert!(router.surrounding_for_prediction().is_none());
}
