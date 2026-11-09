//! 对着一个假 Server 跑握手与按键。
//!
//! 假 Server 只复刻协议的形状（哪条消息有回包、回什么壳），不复刻引擎：
//! 这里要钉住的是**客户端有没有按协议说话**，真引擎的行为由 Server 自己的测试管。

use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::thread;

use qingjian_platform::protocol::linux::{Capabilities, LINUX_UI_PROTOCOL};
use qingjian_platform::protocol::{
    Frame, KeyOutcome, PROTOCOL_VERSION, read_message, write_message,
};
use serde_json::{Value, json};

use super::Connection;
use crate::error::IbusError;

/// 假 Server 收到的消息，测完拿回来核对。
type Received = std::sync::Arc<std::sync::Mutex<Vec<Value>>>;

/// 起一个假 Server，返回 socket 路径与收到的消息。
///
/// `limit` 是「服务几条消息之后就收工关连接」，用来演 Server 中途没了；`None` 是一直服务到客户端断开。
fn fake_server(limit: Option<usize>) -> (PathBuf, Received, thread::JoinHandle<()>) {
    let path = std::env::temp_dir().join(format!(
        "qingjian-ibus-test-{}-{:?}.sock",
        std::process::id(),
        thread::current().id()
    ));
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).expect("绑定假 Server 的 socket");
    let received: Received = Default::default();
    let seen = received.clone();
    let handle = thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let mut served = 0;
        while let Ok(Some(value)) = read_message::<_, Value>(&mut stream) {
            seen.lock().expect("记下收到的消息").push(value.clone());
            served += 1;
            let reply = if value.get("OpenSession").is_some() {
                Some(json!({"Update": {"session": 1, "frame": Frame::default(), "linux_ui": {}}}))
            } else if value.get("LinuxHello").is_some() {
                Some(json!({"LinuxHello": {"session": 1, "page_size": 9}}))
            } else if value.get("LinuxEvent").is_some() {
                // 真 Server 在回包上另附展示身份（identity），轮询靠里面的版本号判断帧换没换
                Some(json!({"KeyResult": {
                    "session": 1,
                    "outcome": KeyOutcome::Consumed,
                    "commit": Value::Null,
                    "frame": Frame::default(),
                    "identity": {"generation": 1, "context": "/ibus/context/1", "revision": 7},
                }}))
            } else if value.get("Poll").is_some() {
                Some(json!({"Update": {
                    "session": 1,
                    "frame": Frame::default(),
                    "identity": {"generation": 1, "context": "/ibus/context/1", "revision": 8},
                }}))
            } else {
                // CloseSession 没有回包
                None
            };
            if let Some(reply) = reply
                && write_message(&mut stream, &reply).is_err()
            {
                return;
            }
            if limit.is_some_and(|limit| served >= limit) {
                return;
            }
        }
    });
    (path, received, handle)
}

/// 整条链路：开会话 → 报能力 → 按键 → 关会话，顺序与字段都要对得上 Server 的要求。
#[test]
fn handshake_then_key_round_trips() {
    let (path, received, server) = fake_server(None);
    let mut connection = Connection::connect_at(&path).expect("连上假 Server");

    let session = connection.open_session("/ibus/context/1").expect("开会话");
    let capabilities = connection
        .capabilities(session, Capabilities::default())
        .expect("报能力");
    assert_eq!(capabilities.outcome, KeyOutcome::Consumed);

    let key = crate::key::to_key_event('n' as u32, 0);
    let reply = connection.key(session, key, false).expect("报按键");
    assert_eq!(reply.outcome, KeyOutcome::Consumed);
    assert!(reply.commit.is_none());

    connection.close_session(session).expect("关会话");
    drop(connection);
    server.join().expect("假 Server 收工");
    let _ = std::fs::remove_file(&path);

    let seen = received.lock().expect("取回收到的消息");
    assert_eq!(seen.len(), 5, "开会话两条 + 能力 + 按键 + 关会话");

    // OpenSession 带的是 Windows 那套协议版本
    assert_eq!(seen[0]["OpenSession"]["protocol"], json!(PROTOCOL_VERSION));

    // LinuxHello：版本、代次、上下文三样 Server 都会查，对不上直接断开
    let hello = &seen[1]["LinuxHello"];
    assert_eq!(hello["version"], json!(LINUX_UI_PROTOCOL));
    assert!(
        hello["generation"].as_u64().expect("代次") > 0,
        "代次要大于 0"
    );
    assert_eq!(hello["context"], json!("/ibus/context/1"));

    // 能力必须排在按键前面：Server 在收到它之前不收按键
    assert!(seen[2]["LinuxEvent"]["event"]["Capabilities"].is_object());
    assert_eq!(
        seen[3]["LinuxEvent"]["event"]["Key"]["event"]["virtual_key"],
        json!('n' as u32)
    );
}

/// 连不上就是连不上，不要在这里重试或者吞掉错误：上层按这个决定要不要放行按键。
#[test]
fn connecting_to_a_dead_socket_fails() {
    let path = std::env::temp_dir().join("qingjian-ibus-test-not-there.sock");
    let _ = std::fs::remove_file(&path);
    assert!(Connection::connect_at(&path).is_err());
}

/// 会话编号在一条连接里递增，Server 靠它区分同一个前端的多个输入上下文。
#[test]
fn sessions_are_numbered_per_connection() {
    let (path, _received, server) = fake_server(None);
    let mut connection = Connection::connect_at(&path).expect("连上假 Server");
    let first = connection.open_session("/ibus/context/1").expect("第一个");
    let second = connection.open_session("/ibus/context/2").expect("第二个");
    assert_ne!(first, second);
    drop(connection);
    server.join().expect("假 Server 收工");
    let _ = std::fs::remove_file(&path);
}

/// 假 Server 关掉之后再发就该报错，而不是卡在读上。
#[test]
fn closed_server_is_reported() {
    // 只服务开会话那两条，之后 Server 就没了
    let (path, _received, server) = fake_server(Some(2));
    let mut connection = Connection::connect_at(&path).expect("连上假 Server");
    let session = connection.open_session("/ibus/context/1").expect("开会话");
    server.join().expect("假 Server 收工");
    let _ = std::fs::remove_file(&path);

    let key = crate::key::to_key_event('n' as u32, 0);
    let error = connection
        .key(session, key, false)
        .expect_err("对端没了要报错");
    assert!(
        matches!(error, IbusError::Closed | IbusError::Codec(_)),
        "应该是连接断了这一类，实际是 {error}"
    );
}

/// 组句期间的轮询：发的是 `Poll`，回包（`Update`）与按键的回包（`KeyResult`）里展示版本号都要读出来，
/// IBus 前端靠它判断本地整句模型的重排结果到了没有（版本号变新才重画）。
#[test]
fn poll_reads_display_revision() {
    let (path, received, server) = fake_server(None);
    let mut connection = Connection::connect_at(&path).expect("连上假 Server");
    let session = connection.open_session("/ibus/context/1").expect("开会话");
    connection
        .capabilities(session, Capabilities::default())
        .expect("报能力");

    let typed = connection
        .key(session, crate::key::to_key_event('n' as u32, 0), false)
        .expect("按键");
    assert_eq!(typed.revision, Some(7));
    let polled = connection.poll(session).expect("轮询");
    assert_eq!(polled.revision, Some(8));

    connection.close_session(session).expect("关会话");
    drop(connection);
    server.join().expect("假 Server 正常收工");
    let seen = received.lock().expect("取回收到的消息");
    assert!(
        seen.contains(&json!({"Poll": {"session": session}})),
        "轮询要发 Poll：{seen:?}"
    );
}
