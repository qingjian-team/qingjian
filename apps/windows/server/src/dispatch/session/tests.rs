//! 会话归属：会话号是 DLL 报上来的线程号，可枚举，所以只有开它的那条连接能动它。

use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_platform::protocol::{
    ClientMessage, KeyEvent, KeyModifiers, PROTOCOL_VERSION, ServerMessage, SessionId,
};

use crate::dispatch::{Router, RouterConfig};

/// 一个随手写的会话号，模拟「DLL 的线程号被别人猜中」。
const FOREIGN: SessionId = SessionId(1234);

fn router() -> Router {
    Router::new(
        Engine::new(Dictionary::parse("你\tni\t100\n").unwrap()),
        RouterConfig::default(),
    )
}

fn open(session: SessionId, app: &str) -> ClientMessage {
    ClientMessage::OpenSession {
        session,
        app: Some(app.to_owned()),
        protocol: PROTOCOL_VERSION,
    }
}

fn key(session: SessionId, virtual_key: u32, character: char) -> ClientMessage {
    ClientMessage::Key {
        session,
        event: KeyEvent::new(virtual_key, Some(character), KeyModifiers::default()),
    }
}

#[test]
fn another_connection_cannot_poll_a_foreign_session() {
    let mut router = router();
    router.handle_from(1, open(FOREIGN, "Code.exe"));
    router.handle_from(1, key(FOREIGN, 0x4E, 'n'));

    // 2 号连接拿同一个会话号问：不给（原来会把这条会话的拼音与候选整页回出去）
    assert!(
        router
            .handle_from(2, ClientMessage::Poll { session: FOREIGN })
            .is_none()
    );
    // 1 号连接照常拿得到
    assert!(matches!(
        router.handle_from(1, ClientMessage::Poll { session: FOREIGN }),
        Some(ServerMessage::Update { .. })
    ));
}

#[test]
fn another_connection_cannot_take_over_a_session() {
    let mut router = router();
    router.handle_from(1, open(FOREIGN, "Code.exe"));

    // 抢开同一个会话号：拒，宿主应用名没被别人改掉
    assert!(router.handle_from(2, open(FOREIGN, "evil.exe")).is_none());
    assert!(
        router
            .handle_from(2, ClientMessage::Commit { session: FOREIGN })
            .is_none()
    );
    // 会话还在，原主照常用
    assert_eq!(router.session_count(), 1);
    assert!(
        router
            .handle_from(1, ClientMessage::Commit { session: FOREIGN })
            .is_some()
    );
}

#[test]
fn unopened_sessions_accept_no_state_messages() {
    let mut router = router();
    // 会话没注册：状态类的消息一律不认——不然别人靠猜会话号就能抢焦点、清组句
    assert!(router.handle_from(7, key(FOREIGN, 0x4E, 'n')).is_none());
    assert!(
        router
            .handle_from(7, ClientMessage::Commit { session: FOREIGN })
            .is_none()
    );
    // 例外：问全局模式的那条（`SyncMode` 只读配置；`ImeSwitched` 走临时连接，也不回话）
    assert!(
        router
            .handle_from(7, ClientMessage::SyncMode { session: FOREIGN })
            .is_some()
    );
    assert_eq!(router.session_count(), 0);
}

#[test]
fn unattended_callers_keep_the_old_behaviour() {
    // 测试的内存流（`ipc::serve`）用 UNATTENDED：行为与加这道门之前一样，谁都能问。
    let mut router = router();
    router.handle(open(FOREIGN, "Code.exe"));
    assert!(matches!(
        router.handle(ClientMessage::Poll { session: FOREIGN }),
        Some(ServerMessage::Update { .. })
    ));
}

/// 切走输入法的通知是 DLL 在 TSF 线程上**现连一条管道**发的，会话开在别的连接上。
/// 按归属一刀切会把这条合法通知丢掉，状态条就永远收不起来——这条用例盯的就是这个回退。
#[test]
fn ime_switched_comes_from_a_temporary_connection() {
    let mut router = router();
    router.handle_from(1, open(FOREIGN, "Code.exe"));
    let switched = ClientMessage::ImeSwitched { session: FOREIGN };
    assert!(router.admits(2, &switched), "临时连接发的通知要放行");
    // 同一条连接上别的消息照旧按归属判，门没被整条撬开
    assert!(
        !router.admits(2, &ClientMessage::Poll { session: FOREIGN }),
        "别人的会话照样问不得"
    );
}

#[test]
fn a_closed_session_is_unregistered_again() {
    let mut router = router();
    router.handle_from(1, open(FOREIGN, "Code.exe"));
    router.handle_from(1, ClientMessage::CloseSession { session: FOREIGN });
    assert_eq!(router.session_count(), 0);
    // 关了之后 2 号连接也拿不到（会话已经不在表里）
    assert!(
        router
            .handle_from(2, ClientMessage::Poll { session: FOREIGN })
            .is_none()
    );
}
