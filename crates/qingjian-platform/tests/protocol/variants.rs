//! 每个协议变体都要能原样往返。
//!
//! 前面 `frames.rs` 验的是「字节流坏了会怎样」，这里验的是另一件事：
//! **每个变体自己都得能编码再解回来**。加新变体时最容易忘的是给它补
//! `#[serde(default)]`、或者漏掉 `Deserialize` 派生——编译照样过，直到某个旧 DLL
//! 发来不带那个字段的消息，Server 才在线上报 JSON 错误。
//!
//! 所以这里是全量枚举，不是挑几个代表。少一个变体就少一道防线，
//! 而「少一个」正是这类测试典型的失效方式。

use std::io::Cursor;

use qingjian_platform::protocol::{
    ClientMessage, Frame, IndicatorCommand, IndicatorState, InputSettings, KeyEvent, KeyModifiers,
    KeyOutcome, PreeditKind, PreeditSegment, ScreenRect, ServerMessage, SessionId, read_message,
    write_message,
};

/// 一个够用的 `Frame`。字段不全填，`Default` 的部分留默认值——
/// 往返后要与原值相等，缺的字段必须两边一致地缺。
fn frame() -> Frame {
    Frame {
        preedit: vec![PreeditSegment {
            text: "ni'hao".to_owned(),
            kind: PreeditKind::Typed,
        }],
        ..Frame::default()
    }
}

fn screen_rect() -> ScreenRect {
    ScreenRect {
        left: 12,
        top: 34,
        right: 56,
        bottom: 78,
    }
}

fn key_event() -> KeyEvent {
    KeyEvent::new(
        0x41,
        Some('a'),
        KeyModifiers {
            shift: true,
            ..KeyModifiers::default()
        },
    )
}

/// DLL → Server 的每一个变体。新加变体时这里会编译不过，那是故意的：
/// 逼着写的人给新变体补一条往返。
fn client_messages() -> Vec<ClientMessage> {
    let session = SessionId(9);
    vec![
        ClientMessage::OpenSession {
            session,
            app: Some("notepad.exe".to_owned()),
            protocol: qingjian_platform::protocol::PROTOCOL_VERSION,
        },
        // 老 DLL 不带 app 与 protocol，两个字段都有 `#[serde(default)]`
        ClientMessage::OpenSession {
            session,
            app: None,
            protocol: 0,
        },
        ClientMessage::Key {
            session,
            event: key_event(),
        },
        ClientMessage::Commit { session },
        ClientMessage::Poll { session },
        ClientMessage::Surrounding {
            session,
            text: "光标前文".to_owned(),
        },
        ClientMessage::Privacy {
            session,
            private: true,
        },
        ClientMessage::Selection {
            session,
            request: 7,
            text: "选中的文字".to_owned(),
            rect: screen_rect(),
        },
        ClientMessage::SelectionFallback {
            session,
            request: 8,
            process: 42,
            window: 0x1234,
            text: String::new(),
            rect: screen_rect(),
        },
        ClientMessage::PositionCandidates {
            session,
            rect: screen_rect(),
        },
        ClientMessage::HideCandidates { session },
        ClientMessage::ModeChanged {
            session,
            english: true,
        },
        ClientMessage::SyncMode { session },
        ClientMessage::ImeSwitched { session },
        ClientMessage::Indicator {
            session,
            command: IndicatorCommand::TogglePunctuation,
        },
        ClientMessage::Indicator {
            session,
            command: IndicatorCommand::OpenSymbolPanel,
        },
        ClientMessage::CloseSession { session },
    ]
}

/// Server → DLL 的每一个变体。
fn server_messages() -> Vec<ServerMessage> {
    let session = SessionId(9);
    vec![
        ServerMessage::SessionOpened {
            session,
            input: InputSettings::default(),
        },
        ServerMessage::KeyResult {
            session,
            outcome: KeyOutcome::Consumed,
            commit: None,
            frame: frame(),
            english: None,
        },
        // `commit` 的 `Some` 侧：上屏时带文字。只测 `None` 的话，`Option` 的序列化形状变了没人发现。
        ServerMessage::KeyResult {
            session,
            outcome: KeyOutcome::Passthrough,
            commit: Some("上屏".to_owned()),
            frame: Frame::default(),
            english: Some(true),
        },
        ServerMessage::Committed {
            session,
            text: Some("缓冲原样交出".to_owned()),
        },
        ServerMessage::Committed {
            session,
            text: None,
        },
        ServerMessage::SelectionReview {
            session,
            active: true,
        },
        ServerMessage::SelectionReview {
            session,
            active: false,
        },
        ServerMessage::Update {
            session,
            frame: frame(),
            commit: None,
        },
        ServerMessage::ModeSync {
            session,
            english: Some(false),
            input: InputSettings::default(),
            indicator: IndicatorState::default(),
        },
        ServerMessage::ModeSync {
            session,
            english: Some(true),
            input: InputSettings::default(),
            indicator: IndicatorState::default(),
        },
        ServerMessage::RequestSelection {
            session,
            request: 3,
        },
    ]
}

/// 逐条往返：写进字节流、读回来、与原值比。
///
/// 走的是真的 `write_message` / `read_message`（长度前缀 + JSON），不是
/// `serde_json::to_string` / `from_str`——那会把帧格式排除在外，
/// 而长度前缀与消息体对不上正是这里要防的。
fn round_trips<T>(messages: &[T])
where
    T: PartialEq + std::fmt::Debug + serde::Serialize + serde::de::DeserializeOwned,
{
    for (i, message) in messages.iter().enumerate() {
        let mut buf = Vec::new();
        write_message(&mut buf, message).unwrap_or_else(|e| panic!("第 {i} 条写不进去：{e}"));
        let mut cursor = Cursor::new(buf);
        let got: Option<T> =
            read_message(&mut cursor).unwrap_or_else(|e| panic!("第 {i} 条读不回来：{e}"));
        assert_eq!(
            got.as_ref(),
            Some(message),
            "第 {i} 条往返后变了：写进去 {message:?}，读出来 {got:?}"
        );
        // 一帧读完就该到底：帧边界没有多读也没有少读
        assert_eq!(
            cursor.position() as usize,
            cursor.get_ref().len(),
            "第 {i} 条之后还有字节，帧边界错了"
        );
    }
}

#[test]
fn every_client_message_round_trips() {
    round_trips(&client_messages());
}

#[test]
fn every_server_message_round_trips() {
    round_trips(&server_messages());
}

#[test]
fn an_old_dll_without_the_protocol_field_still_opens_a_session() {
    // `OpenSession.protocol` 带 `#[serde(default)]`，老 DLL 不带这个字段。
    // 这条验的正是那个默认值：少了字段不能读成错误，否则升级安装后
    // 「旧 DLL 留在没重启的应用里」直接开不了会话。
    let body = br#"{"OpenSession":{"session":9}}"#;
    let mut bytes = (body.len() as u32).to_le_bytes().to_vec();
    bytes.extend_from_slice(body);
    let mut cursor = Cursor::new(bytes);
    let got: Option<ClientMessage> = read_message(&mut cursor).expect("少字段也该读得出来");
    assert_eq!(
        got,
        Some(ClientMessage::OpenSession {
            session: SessionId(9),
            app: None,
            protocol: 0,
        }),
        "老 DLL 的 OpenSession 应按 protocol=0 读入"
    );
}

#[test]
fn a_frame_without_the_translation_pos_field_still_parses() {
    // 新增的随帧开关与 aux_code_show 同款：旧端不带这个字段的帧必须照常解出来，
    // 缺省走普通 bool 缺省（与 Frame::default 一致）；显式给值时以值为准。
    // 帧的渲染消费者只有 Server 自己的候选窗（DLL 不读该字段），所以缺省值不承担
    // 「跨版本保持显示」的职责，只要不破坏旧帧解析即可。
    let mut json = serde_json::to_value(Frame::default()).expect("序列化默认帧");
    json.as_object_mut()
        .expect("帧是 JSON 对象")
        .remove("translation_pos");
    let frame: Frame = serde_json::from_value(json).expect("缺 translation_pos 的旧帧也该读得出来");
    assert_eq!(frame.translation_pos, Frame::default().translation_pos);

    for value in [true, false] {
        let mut json = serde_json::to_value(Frame::default()).expect("序列化默认帧");
        json.as_object_mut()
            .expect("帧是 JSON 对象")
            .insert("translation_pos".to_owned(), serde_json::json!(value));
        let frame: Frame = serde_json::from_value(json).expect("带字段的帧也该读得出来");
        assert_eq!(frame.translation_pos, value);
    }
}

#[test]
fn an_unknown_field_is_ignored_rather_than_rejected() {
    // 将来加字段时，新 Server 读旧 DLL 不带新字段的消息、以及反向的兼容，
    // 都靠「未知字段忽略」这条。写死在这里，免得有人哪天给某个变体加了
    // `#[serde(deny_unknown_fields)]` 而没人发现线序被破坏。
    let body = br#"{"Poll":{"session":9,"__some_future_field__":1}}"#;
    let mut bytes = (body.len() as u32).to_le_bytes().to_vec();
    bytes.extend_from_slice(body);
    let mut cursor = Cursor::new(bytes);
    let got: Option<ClientMessage> = read_message(&mut cursor).expect("未知字段不该让整条消息失败");
    assert_eq!(
        got,
        Some(ClientMessage::Poll {
            session: SessionId(9)
        })
    );
}

const PROBE: &str = "__probe__";

/// 从 serde 的「unknown variant」错误里把真实变体名列表抠出来。
///
/// 这是这里唯一**自动**的部分：main 加了新变体，错误信息里就多一个名字，
/// 与下面的覆盖清单比对不上，测试就红。手写一份清单再比条数是没用的——
/// 加变体的人不会知道要同步哪个数字。
fn declared_variants<T: serde::de::DeserializeOwned + std::fmt::Debug>() -> Vec<String> {
    let err = serde_json::from_str::<T>(r#"{"__probe__":{}}"#)
        .expect_err("不存在的变体名应该报 unknown variant");
    let text = err.to_string();
    let start = text.find('`').expect("错误信息里应有变体名列表");
    let list = &text[start..];
    list.split('`')
        .skip(1) // 第一个是 `` 前的内容
        .step_by(2) // 隔一个取：变体名在奇数位
        .filter(|name| *name != PROBE) // serde 把探针名自己也列进候选
        .map(str::to_owned)
        .collect()
}

/// 一条消息覆盖了哪个变体名（externally tagged，tag 就是 JSON 的键）。
fn tag_of<T: serde::Serialize>(message: &T) -> String {
    let json = serde_json::to_string(message).expect("序列化");
    let value: serde_json::Value = serde_json::from_str(&json).expect("是 JSON");
    value
        .as_object()
        .and_then(|object| object.keys().next().cloned())
        .expect("externally tagged 的 JSON 应以变体名为键")
}

#[test]
fn every_declared_variant_is_covered_by_a_round_trip() {
    let declared = declared_variants::<ClientMessage>();
    let covered: Vec<String> = client_messages().iter().map(tag_of).collect();
    let missing: Vec<&String> = declared
        .iter()
        .filter(|name| !covered.contains(name))
        .collect();
    assert!(
        missing.is_empty(),
        "ClientMessage 有变体没有往返条目：{missing:?}。加新变体时请在 client_messages() 里补一条"
    );

    let declared = declared_variants::<ServerMessage>();
    let covered: Vec<String> = server_messages().iter().map(tag_of).collect();
    let missing: Vec<&String> = declared
        .iter()
        .filter(|name| !covered.contains(name))
        .collect();
    assert!(
        missing.is_empty(),
        "ServerMessage 有变体没有往返条目：{missing:?}。加新变体时请在 server_messages() 里补一条"
    );
}
