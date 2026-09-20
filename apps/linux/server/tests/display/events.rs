//! 错误鼠标身份不改变其他会话曝光，重开上下文不会复用旧帧。
use super::{compose, key, router};
use qingjian_platform::protocol::{ClientMessage, PROTOCOL_VERSION, SessionId};
use serde_json::json;

#[test]
fn rejected_other_session_action_keeps_current_exposure_eligible() {
    for action in ["Candidate", "Page"] {
        let (mut router, book) = router();
        router.handle(ClientMessage::OpenSession {
            session: SessionId(2),
            app: None,
            protocol: PROTOCOL_VERSION,
        });
        router.handle_linux(json!({"DisplayReporting": {"session": 2, "identity": {"generation": 2, "context": "second", "revision": 0}}}));
        let identity = compose(&mut router);
        let event = if action == "Candidate" {
            json!({"Candidate": {"identity": identity, "index": 0}})
        } else {
            json!({"Page": {"identity": identity, "next": true}})
        };
        let ignored = router
            .handle_linux(json!({"LinuxEvent": {"session": 2, "event": event}}))
            .unwrap();
        assert_eq!(ignored["Ignored"]["session"], 2);
        router.handle_linux(json!({"DisplayAcknowledged": {"session": 1, "identity": identity, "senses": [[0, 0]]}}));
        assert_eq!(key(&mut router, ' ')["KeyResult"]["commit"], "你好");
        assert_eq!(*book.lock().unwrap(), ["hello"], "{action}");
    }
}
#[test]
fn reopened_context_rejects_old_revision_even_with_same_local_id() {
    let (mut router, book) = router();
    let old = compose(&mut router);
    router.handle(ClientMessage::CloseSession {
        session: SessionId(1),
    });
    router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    router.handle(ClientMessage::Privacy {
        session: SessionId(1),
        private: false,
    });
    router.handle_linux(json!({"DisplayReporting": {"session": 1, "identity": {"generation": 2, "context": "first", "revision": 0}}}));
    let new = compose(&mut router);
    assert_ne!(old, new);
    let ignored = router.handle_linux(json!({"LinuxEvent": {"session": 1, "event": {"Candidate": {"identity": old, "index": 0}}}})).unwrap();
    assert_eq!(ignored["Ignored"]["session"], 1);
    router.handle_linux(
        json!({"DisplayAcknowledged": {"session": 1, "identity": old, "senses": [[0, 0]]}}),
    );
    key(&mut router, ' ');
    assert!(book.lock().unwrap().is_empty());
}
