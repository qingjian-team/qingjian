//! 同一真实 socket 的 128 会话、逐会话身份、回收及错误握手。
#![cfg(target_os = "linux")]
#[path = "support/server.rs"]
mod server;
use qingjian_platform::protocol::{PROTOCOL_VERSION, read_message, write_message};
use serde_json::{Value, json};
use server::Server;
use std::os::unix::net::UnixStream;

fn exchange(stream: &mut UnixStream, request: Value) -> Value {
    write_message(stream, &request).unwrap();
    read_message::<_, Value>(stream).unwrap().unwrap()
}
fn open(stream: &mut UnixStream, id: u64) {
    let response = exchange(
        stream,
        json!({"OpenSession": {"session": id, "app": "multiplex", "protocol": PROTOCOL_VERSION}}),
    );
    assert_eq!(response["Update"]["linux_ui"]["version"], 3);
    let hello = exchange(
        stream,
        json!({"LinuxHello": {"version": 3, "session": id, "generation": 1, "context": format!("context-{id}")}}),
    );
    assert_eq!(hello["LinuxHello"]["session"], id);
    let acknowledged = exchange(
        stream,
        json!({"LinuxEvent": {"session": id, "event": {"Capabilities": {"sensitive": false, "password": false, "disabled": false}}}}),
    );
    assert_eq!(acknowledged["KeyResult"]["session"], id);
}
fn key(stream: &mut UnixStream, id: u64, character: char) -> Value {
    exchange(
        stream,
        json!({"LinuxEvent": {"session": id, "event": {"Key": {"event": {"virtual_key": character as u32, "character": character, "modifiers": {"ctrl": false, "shift": false, "alt": false, "win": false, "caps": false, "english_mode": false}}, "release": false}}}}),
    )
}
#[test]
fn simultaneous_sessions_keep_buffers_and_survive_retirement() {
    let directory = std::env::temp_dir().join(format!("qingjian-multiplex-{}", std::process::id()));
    let mut server = Server::start(directory.clone());
    let mut stream = server.connect();
    for id in 1..=128 {
        open(&mut stream, id);
        for c in if id % 2 == 0 { "hao" } else { "ni" }.chars() {
            key(&mut stream, id, c);
        }
    }
    for id in 1..=128 {
        let response = key(&mut stream, id, ' ');
        assert_eq!(response["KeyResult"]["session"], id);
        assert_eq!(
            response["KeyResult"]["commit"],
            if id % 2 == 0 { "好" } else { "你" }
        );
        assert_eq!(
            response["KeyResult"]["identity"]["context"],
            format!("context-{id}")
        );
    }
    let a = key(&mut stream, 1, 'n');
    key(&mut stream, 2, 'h');
    // A 帧不能对 B 选词，合法旧事件也不能破坏 A/B 的输入状态。
    let crossed = exchange(
        &mut stream,
        json!({"LinuxEvent": {"session": 2, "event": {"Candidate": {"identity": a["KeyResult"]["identity"], "index": 0}}}}),
    );
    assert_eq!(crossed["Ignored"]["session"], 2);
    assert_eq!(
        exchange(&mut stream, json!({"Commit": {"session": 2}}))["Committed"]["text"],
        "h"
    );
    for id in 1..=128 {
        write_message(&mut stream, &json!({"CloseSession": {"session": id}})).unwrap();
    }
    // 重复 Close 无响应，后续事务仍能正确读取自己一份答复。
    write_message(&mut stream, &json!({"CloseSession": {"session": 1}})).unwrap();
    let ignored = exchange(
        &mut stream,
        json!({"LinuxEvent": {"session": 1, "event": {"Candidate": {"identity": a["KeyResult"]["identity"], "index": 0}}}}),
    );
    assert_eq!(ignored["Ignored"]["session"], 1);
    for id in 129..=320 {
        open(&mut stream, id);
        key(&mut stream, id, 'n');
        write_message(&mut stream, &json!({"CloseSession": {"session": id}})).unwrap();
    }
    open(&mut stream, 321);
    assert!(
        exchange(&mut stream, json!({"Commit": {"session": 321}}))["Committed"]["text"].is_null()
    );
    drop(stream);
    drop(server);
    std::fs::remove_dir_all(directory).unwrap();
}
#[test]
fn rejects_old_missing_duplicate_and_unnegotiated_messages() {
    let directory = std::env::temp_dir().join(format!("qingjian-reject-{}", std::process::id()));
    let mut server = Server::start(directory.clone());
    for scenario in 0..5 {
        let mut stream = server.connect();
        exchange(
            &mut stream,
            json!({"OpenSession": {"session": 1, "app": null, "protocol": PROTOCOL_VERSION}}),
        );
        let hello =
            json!({"LinuxHello": {"version": 3, "session": 1, "generation": 1, "context": "a"}});
        let mut invalid = hello.clone();
        match scenario {
            0 => invalid["LinuxHello"]["version"] = json!(2),
            1 => {
                invalid["LinuxHello"]
                    .as_object_mut()
                    .unwrap()
                    .remove("session");
            }
            2 => {
                exchange(&mut stream, hello);
            }
            3 => {
                invalid = json!({"LinuxEvent": {"session": 1, "event": "Reset"}});
            }
            _ => {
                exchange(&mut stream, hello);
                invalid = json!({"LinuxEvent": {"session": 1, "event": "Reset"}});
            }
        }
        write_message(&mut stream, &invalid).unwrap();
        assert!(
            read_message::<_, Value>(&mut stream).unwrap().is_none(),
            "scenario {scenario}"
        );
    }
    // 损坏客户端只影响自己的连接。
    let mut good = server.connect();
    open(&mut good, 1);
    let mut bad = server.connect();
    use std::io::Write;
    bad.write_all(&[0xff, 0xff, 0xff, 0x7f]).unwrap();
    assert!(read_message::<_, Value>(&mut bad).unwrap().is_none());
    assert_eq!(key(&mut good, 1, 'n')["KeyResult"]["session"], 1);
    drop(server);
    std::fs::remove_dir_all(directory).unwrap();
}
