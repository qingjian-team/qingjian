//! 传输层测试：帧编解码，以及在内存流上跑 serve 循环；不碰命名管道，跨平台可跑。

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use qingjian_core::Language;
use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, PROTOCOL_VERSION, ScreenRect, ServerMessage, SessionId,
};
use qingjian_render::Mode;
use qingjian_windows_server::dispatch::{CandidateSink, KeyHintView, PanelFrame, RenderSettings};
use qingjian_windows_server::ipc::{read_message, serve, write_message};
use qingjian_windows_server::{AssemblySpec, Router, RouterConfig, assembly};

const SESSION: SessionId = SessionId(1);

fn router() -> Router {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let engine = assembly::assemble(&AssemblySpec {
        glossary: Some((
            Language::English,
            root.join("assets/sample/glossary-en.tsv"),
        )),
        ..AssemblySpec::new(root.join("assets/sample/dict.tsv"))
    })
    .expect("assemble engine from sample data");
    Router::new(engine, RouterConfig::default())
}

fn letter(c: char) -> KeyEvent {
    KeyEvent::new(c.to_ascii_uppercase() as u32, Some(c), Default::default())
}

#[test]
fn codec_round_trips_a_message() {
    let original = ClientMessage::Key {
        session: SESSION,
        event: letter('n'),
    };
    let mut buffer = Vec::new();
    write_message(&mut buffer, &original).unwrap();

    let mut reader = Cursor::new(buffer);
    let decoded: Option<ClientMessage> = read_message(&mut reader).unwrap();
    assert_eq!(decoded, Some(original));
    let end: Option<ClientMessage> = read_message(&mut reader).unwrap();
    assert_eq!(end, None);
}

#[test]
fn serve_runs_the_open_type_loop_over_a_stream() {
    let mut input = Vec::new();
    write_message(
        &mut input,
        &ClientMessage::OpenSession {
            session: SESSION,
            app: None,
            protocol: PROTOCOL_VERSION,
        },
    )
    .unwrap();
    for c in "nihao".chars() {
        write_message(
            &mut input,
            &ClientMessage::Key {
                session: SESSION,
                event: letter(c),
            },
        )
        .unwrap();
    }

    let mut router = router();
    let mut stream = Duplex {
        inbound: Cursor::new(input),
        outbound: Vec::new(),
    };
    serve(&mut stream, &mut router).unwrap();

    let mut responses = Vec::new();
    let mut out = Cursor::new(stream.outbound);
    while let Some(message) = read_message::<_, ServerMessage>(&mut out).unwrap() {
        responses.push(message);
    }
    // 开会话先回一条 `SessionOpened`（把按键行为设置带下来），之后五个按键各回一条 `KeyResult`。
    assert_eq!(responses.len(), 6, "一条 SessionOpened 加五条 KeyResult");
    assert!(
        matches!(responses.first(), Some(ServerMessage::SessionOpened { .. })),
        "首条应是 SessionOpened，实际：{:?}",
        responses.first()
    );

    let ServerMessage::KeyResult { frame, .. } = responses.last().unwrap() else {
        panic!("末条应是 KeyResult");
    };
    let preedit: String = frame.preedit.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(preedit, "ni'hao");
    let texts: Vec<&str> = frame
        .candidates
        .items
        .iter()
        .map(|c| c.text.as_str())
        .collect();
    assert!(
        texts.contains(&"你好"),
        "候选里应有「你好」，实际：{texts:?}"
    );
}

/// 开会话 → 敲 `nihao` → 报光标矩形 → 对端就这么没了（不发 `CloseSession`）的输入流。
fn composing_then_vanished() -> Vec<u8> {
    let mut input = Vec::new();
    write_message(
        &mut input,
        &ClientMessage::OpenSession {
            session: SESSION,
            app: None,
            protocol: PROTOCOL_VERSION,
        },
    )
    .unwrap();
    write_message(
        &mut input,
        &ClientMessage::Key {
            session: SESSION,
            event: letter('n'),
        },
    )
    .unwrap();
    // 光标矩形要等会话聚焦了才作数（先聚焦才有候选窗口可显示）
    write_message(
        &mut input,
        &ClientMessage::PositionCandidates {
            session: SESSION,
            rect: ScreenRect {
                left: 0,
                top: 0,
                right: 100,
                bottom: 20,
            },
        },
    )
    .unwrap();
    for c in "ihao".chars() {
        write_message(
            &mut input,
            &ClientMessage::Key {
                session: SESSION,
                event: letter(c),
            },
        )
        .unwrap();
    }
    input
}

/// 应用被关 / 崩了 / 管道断：DLL 来不及发 `CloseSession`，Server 得自己照单收掉本连接开过的会话。
/// 留在 Router 里的聚焦会话会让组句与候选窗口一直挂着，直到别的应用来一个键把它挤掉。
#[test]
fn dropping_a_connection_closes_its_session() {
    let mut router = router();
    let mut stream = Duplex {
        inbound: Cursor::new(composing_then_vanished()),
        outbound: Vec::new(),
    };
    serve(&mut stream, &mut router).unwrap();

    assert_eq!(
        router.session_count(),
        0,
        "连接断开后会话要收掉，否则聚焦会话一直占着 Router"
    );
    assert!(
        router.engine_mut().composition().is_empty(),
        "连接断开后缓冲要清干净，否则下一个应用接着敲会接在旧拼音后面"
    );
}

/// 同上，但验的是用户看得见的那半个：挂在屏幕上的候选窗口要收掉。
///
/// 按**顺序**判，不数次数：第一个键的 `ensure_focus` 本身就会收一次窗（聚焦切换），
/// 数「收过几次」的话没修也恒大于零，那是道假绿灯。
#[test]
fn dropping_the_focused_connection_leaves_the_candidate_window_hidden() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut router = router();
    router.set_candidate_sink(Box::new(RecordingSink {
        events: Arc::clone(&events),
    }));
    let mut stream = Duplex {
        inbound: Cursor::new(composing_then_vanished()),
        outbound: Vec::new(),
    };
    serve(&mut stream, &mut router).unwrap();

    let events = events.lock().unwrap().clone();
    assert!(
        events.contains(&"show"),
        "组句期间候选窗口该显示过，否则这条没验到东西：{events:?}"
    );
    assert_eq!(
        events.last().copied(),
        Some("hide"),
        "连接断开后候选窗口要收着，不能还挂在屏幕上：{events:?}"
    );
}

/// 另一个应用正在组句的会话。Router 里的号由 Server 分配、从 1 往上数，取个分配不到的大数，
/// 模拟攻击方从线程 id 猜到的号恰好就是它。
const VICTIM: SessionId = SessionId(u64::MAX - 1);

/// 让 `VICTIM` 在 Router 里聚焦并组着 `ni`（不经管道，攻击方看不到它的连接）。
fn victim_composing(router: &mut Router) {
    router.handle(ClientMessage::OpenSession {
        session: VICTIM,
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    for c in "ni".chars() {
        router.handle(ClientMessage::Key {
            session: VICTIM,
            event: letter(c),
        });
    }
    assert_eq!(router.engine_mut().composition().text(), "ni");
}

/// 把输入流喂给 `serve`，收回所有回包。
fn serve_messages(router: &mut Router, messages: &[ClientMessage]) -> Vec<ServerMessage> {
    let mut input = Vec::new();
    for message in messages {
        write_message(&mut input, message).unwrap();
    }
    let mut stream = Duplex {
        inbound: Cursor::new(input),
        outbound: Vec::new(),
    };
    serve(&mut stream, router).unwrap();
    let mut out = Cursor::new(stream.outbound);
    let mut responses = Vec::new();
    while let Some(message) = read_message::<_, ServerMessage>(&mut out).unwrap() {
        responses.push(message);
    }
    responses
}

/// #699：没开过会话的连接拿别人的会话号 `Poll` / `Commit` / 按键，读不到也动不了别人的组句，连接直接断开。
#[test]
fn a_connection_cannot_touch_a_session_it_never_opened() {
    let mut router = router();
    victim_composing(&mut router);

    let responses = serve_messages(
        &mut router,
        &[
            ClientMessage::Poll { session: VICTIM },
            ClientMessage::Commit { session: VICTIM },
            ClientMessage::Key {
                session: VICTIM,
                event: letter('a'),
            },
        ],
    );

    assert!(responses.is_empty(), "不该回任何东西：{responses:?}");
    assert_eq!(
        router.engine_mut().composition().text(),
        "ni",
        "别的应用的组句不能被读走或清掉"
    );
    assert_eq!(router.session_count(), 1);
}

/// #699：自己开一个和别人同号的会话，拿到的是自己那个新会话，看不到别人的组句。
#[test]
fn opening_a_colliding_session_number_gets_a_separate_session() {
    let mut router = router();
    victim_composing(&mut router);

    let responses = serve_messages(
        &mut router,
        &[
            ClientMessage::OpenSession {
                session: VICTIM,
                app: None,
                protocol: PROTOCOL_VERSION,
            },
            ClientMessage::Poll { session: VICTIM },
        ],
    );

    let [
        ServerMessage::SessionOpened { session, .. },
        ServerMessage::Update { frame, .. },
    ] = responses.as_slice()
    else {
        panic!("应回 SessionOpened 与 Update：{responses:?}");
    };
    assert_eq!(*session, VICTIM, "回包里的会话号要换回 DLL 报的那个");
    assert!(frame.preedit.is_empty(), "不能看到别人的组句：{frame:?}");
    assert_eq!(router.engine_mut().composition().text(), "ni");
    assert_eq!(
        router.session_count(),
        1,
        "连接断开只关自己开的会话，别人的还在"
    );
}

/// 同一条连接重复开同一个会话不是 DLL 会做的事：断开，之后的按键不处理。
#[test]
fn reopening_a_session_on_the_same_connection_disconnects() {
    let mut router = router();
    let open = ClientMessage::OpenSession {
        session: SESSION,
        app: None,
        protocol: PROTOCOL_VERSION,
    };
    let responses = serve_messages(
        &mut router,
        &[
            open.clone(),
            open,
            ClientMessage::Key {
                session: SESSION,
                event: letter('n'),
            },
        ],
    );
    assert_eq!(responses.len(), 1, "只回第一次开会话：{responses:?}");
    assert_eq!(router.session_count(), 0);
}

/// DLL 被停用后用临时连接发 `ImeSwitched`，不开会话；这条要照常收，连接不断。
#[test]
fn ime_switched_on_a_temporary_connection_is_accepted() {
    let mut router = router();
    let responses = serve_messages(
        &mut router,
        &[
            ClientMessage::ImeSwitched { session: SESSION },
            ClientMessage::OpenSession {
                session: SESSION,
                app: None,
                protocol: PROTOCOL_VERSION,
            },
        ],
    );
    assert!(
        matches!(responses.as_slice(), [ServerMessage::SessionOpened { .. }]),
        "ImeSwitched 之后连接还在、能开会话：{responses:?}"
    );
}

/// 按顺序记候选窗口是显示还是收起。
struct RecordingSink {
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl CandidateSink for RecordingSink {
    fn show(&self, _frame: Frame, _mode: Mode, _rect: ScreenRect, _key_hint: Option<KeyHintView>) {
        self.events.lock().unwrap().push("show");
    }

    fn hide(&self) {
        self.events.lock().unwrap().push("hide");
    }

    fn show_panel(&self, _frame: PanelFrame, _rect: Option<ScreenRect>) {}

    fn hide_panel(&self) {}

    fn configure(&self, _settings: RenderSettings) {}
}

/// 预置输入 + 输出缓冲拼成的流（先发后收够测）。
struct Duplex {
    inbound: Cursor<Vec<u8>>,
    outbound: Vec<u8>,
}

impl Read for Duplex {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inbound.read(buf)
    }
}

impl Write for Duplex {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.outbound.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.outbound.flush()
    }
}
