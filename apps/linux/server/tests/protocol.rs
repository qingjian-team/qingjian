//! 共享长度前缀协议的内存回环。
use qingjian_platform::protocol::{
    ClientMessage, KeyEvent, KeyModifiers, PROTOCOL_VERSION, ServerMessage, SessionId,
    read_message, write_message,
};
use std::io::Cursor;

#[test]
fn client_key_round_trips_with_length_prefix() {
    let message = ClientMessage::Key {
        session: SessionId(42),
        event: KeyEvent::new(u32::from(b'N'), Some('n'), KeyModifiers::default()),
    };
    let mut bytes = Vec::new();
    write_message(&mut bytes, &message).unwrap();
    assert_eq!(
        u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize,
        bytes.len() - 4
    );
    let decoded = read_message::<_, ClientMessage>(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(decoded, Some(message));
}

#[test]
fn open_session_carries_protocol_and_close_is_replyless() {
    let open = ClientMessage::OpenSession {
        session: SessionId(1),
        app: Some("gedit".into()),
        protocol: PROTOCOL_VERSION,
    };
    let close = ClientMessage::CloseSession {
        session: SessionId(1),
    };
    let mut bytes = Vec::new();
    write_message(&mut bytes, &open).unwrap();
    write_message(&mut bytes, &close).unwrap();
    let mut cursor = Cursor::new(bytes);
    assert_eq!(
        read_message::<_, ClientMessage>(&mut cursor).unwrap(),
        Some(open)
    );
    assert_eq!(
        read_message::<_, ClientMessage>(&mut cursor).unwrap(),
        Some(close)
    );
    let response = ServerMessage::Committed {
        session: SessionId(1),
        text: Some("你好".into()),
    };
    let mut out = Vec::new();
    write_message(&mut out, &response).unwrap();
    assert_eq!(
        read_message::<_, ServerMessage>(&mut Cursor::new(out)).unwrap(),
        Some(response)
    );
}

#[test]
fn linux_display_identity_round_trips_without_changing_shared_protocol() {
    let identity = qingjian_linux_server::protocol::DisplayIdentity {
        generation: 7,
        context: "001122".into(),
        revision: 13,
    };
    let ack = qingjian_linux_server::protocol::DisplayAcknowledged {
        session: SessionId(2),
        identity: identity.clone(),
        senses: vec![(0, 1), (3, 0)],
    };
    let value = serde_json::to_value(&ack).unwrap();
    let decoded: qingjian_linux_server::protocol::DisplayAcknowledged =
        serde_json::from_value(value).unwrap();
    assert_eq!(decoded.identity, identity);
    assert_eq!(decoded.senses, vec![(0, 1), (3, 0)]);
}
