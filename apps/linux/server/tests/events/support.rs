//! Linux 事件测试构造器。
use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_linux_server::{Router, RouterConfig};
use qingjian_platform::protocol::{
    ClientMessage, KeyEvent, KeyModifiers, PROTOCOL_VERSION, SessionId,
};
use serde_json::{Value, json};

pub fn router() -> Router {
    let mut router = Router::new(
        Engine::new(
            Dictionary::parse("你好\tni hao\t100\n你\tni\t80\n泥\tni\t70\n拟\tni\t60\n").unwrap(),
        ),
        RouterConfig {
            page_size: 1,
            ..Default::default()
        },
    );
    for id in [1, 2] {
        router.handle(ClientMessage::OpenSession {
            session: SessionId(id),
            app: None,
            protocol: PROTOCOL_VERSION,
        });
        router.handle_linux(json!({"DisplayReporting": {"session": id, "identity": {"generation": 1, "context": format!("context-{id}"), "revision": 0}}}));
        caps(&mut router, id, false, false, false);
    }
    router
}
pub fn event(router: &mut Router, id: u64, event: Value) -> Value {
    router
        .handle_linux(json!({"LinuxEvent": {"session": id, "event": event}}))
        .unwrap()["KeyResult"]
        .clone()
}
pub fn caps(
    router: &mut Router,
    id: u64,
    sensitive: bool,
    password: bool,
    disabled: bool,
) -> Value {
    event(
        router,
        id,
        json!({"Capabilities": {"sensitive": sensitive, "password": password, "disabled": disabled}}),
    )
}
pub fn key(
    router: &mut Router,
    id: u64,
    code: u32,
    character: Option<char>,
    release: bool,
) -> Value {
    event(
        router,
        id,
        json!({"Key": {"event": KeyEvent::new(code, character, KeyModifiers::default()), "release": release}}),
    )
}
pub fn compose(router: &mut Router, id: u64, text: &str) -> Value {
    let mut response = Value::Null;
    for c in text.chars() {
        response = key(router, id, c as u32, Some(c), false);
    }
    response
}
