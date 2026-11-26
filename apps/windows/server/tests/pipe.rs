//! 真命名管道的端到端测试（仅 Windows）：监听线程 + 文件句柄客户端，走完「开会话 → 敲 nihao → 收候选」。

#![cfg(windows)]

use std::fs::OpenOptions;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use qingjian_core::Language;
use qingjian_platform::protocol::{
    ClientMessage, KeyEvent, PROTOCOL_VERSION, ServerMessage, SessionId,
};
use qingjian_windows_server::ipc::pipe::serve_pipe;
use qingjian_windows_server::ipc::{read_message, write_message};
use qingjian_windows_server::{AssemblySpec, Router, RouterConfig, assembly};

const SESSION: SessionId = SessionId(1);

fn letter(c: char) -> KeyEvent {
    KeyEvent::new(c.to_ascii_uppercase() as u32, Some(c), Default::default())
}

/// 客户端重试打开管道，直到监听线程建好实例。
fn connect(name: &str) -> std::fs::File {
    for _ in 0..50 {
        match OpenOptions::new().read(true).write(true).open(name) {
            Ok(file) => return file,
            Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }
    panic!("连不上管道 {name}");
}

/// 起一个监听 `name` 的 Server 线程；它服务完一个客户端就阻塞等下一个，随进程退出即可。
fn spawn_server(name: &str) {
    let server_name = name.to_owned();
    thread::spawn(move || {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let engine = assembly::assemble(&AssemblySpec {
            glossary: Some((
                Language::English,
                root.join("assets/sample/glossary-en.tsv"),
            )),
            ..AssemblySpec::new(root.join("assets/sample/dict.tsv"))
        })
        .expect("assemble engine from sample data");
        let mut router = Router::new(engine, RouterConfig::default());
        let (work_tx, work_rx) = std::sync::mpsc::channel();
        let _ = serve_pipe(&server_name, None, &mut router, work_tx, work_rx);
    });
}

/// #699：两条真管道连接报同一个会话号（DLL 用线程号，别的进程猜得到），后开的只能拿到自己的会话；
/// 不开会话直接来问的，连接被断开。
#[test]
fn two_pipe_connections_with_the_same_session_number_stay_apart() {
    let name = format!(r"\\.\pipe\qingjian-test-cross-{}", std::process::id());
    spawn_server(&name);

    // 应用 A：开会话、敲下 `n`，正组着拼音。
    let mut a = connect(&name);
    let open = ClientMessage::OpenSession {
        session: SESSION,
        app: None,
        protocol: PROTOCOL_VERSION,
    };
    write_message(&mut a, &open).unwrap();
    let opened: ServerMessage = read_message(&mut a).unwrap().expect("A 的 SessionOpened");
    assert!(matches!(opened, ServerMessage::SessionOpened { .. }));
    write_message(
        &mut a,
        &ClientMessage::Key {
            session: SESSION,
            event: letter('n'),
        },
    )
    .unwrap();
    let reply: ServerMessage = read_message(&mut a).unwrap().expect("A 的 KeyResult");
    let ServerMessage::KeyResult { frame, .. } = reply else {
        panic!("A 的首键应回 KeyResult");
    };
    let preedit: String = frame.preedit.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(preedit, "n");

    // 攻击方 B：同一个会话号开会话，再 Poll 同一个号。
    let mut b = connect(&name);
    write_message(&mut b, &open).unwrap();
    let opened: ServerMessage = read_message(&mut b).unwrap().expect("B 的 SessionOpened");
    assert!(matches!(opened, ServerMessage::SessionOpened { .. }));
    write_message(&mut b, &ClientMessage::Poll { session: SESSION }).unwrap();
    let reply: ServerMessage = read_message(&mut b).unwrap().expect("B 的 Update");
    let ServerMessage::Update { frame, .. } = reply else {
        panic!("B 应回 Update");
    };
    assert!(frame.preedit.is_empty(), "B 不能看到 A 的拼音：{frame:?}");
    assert!(
        frame.candidates.items.is_empty(),
        "B 不能看到 A 的候选：{frame:?}"
    );

    // A 没被影响：接着敲 `i`，组句还是自己的。
    write_message(
        &mut a,
        &ClientMessage::Key {
            session: SESSION,
            event: letter('i'),
        },
    )
    .unwrap();
    let reply: ServerMessage = read_message(&mut a).unwrap().expect("A 的第二条 KeyResult");
    let ServerMessage::KeyResult { frame, .. } = reply else {
        panic!("A 的第二键应回 KeyResult");
    };
    let preedit: String = frame.preedit.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(preedit, "ni", "B 开会话不能抢走 A 的组句");

    // 攻击方 C：不开会话就问别人的号，连接被断开，也拿不到回包。
    let mut c = connect(&name);
    write_message(&mut c, &ClientMessage::Poll { session: SESSION }).unwrap();
    let answer = read_message::<_, ServerMessage>(&mut c);
    assert!(
        !matches!(&answer, Ok(Some(_))),
        "没开会话就来问的连接要被断开"
    );
}

#[test]
fn named_pipe_round_trips_the_open_type_loop() {
    let name = format!(r"\\.\pipe\qingjian-test-{}", std::process::id());

    // 监听线程服务完一个客户端后阻塞等下一个，随进程退出即可。
    let server_name = name.clone();
    thread::spawn(move || {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let engine = assembly::assemble(&AssemblySpec {
            glossary: Some((
                Language::English,
                root.join("assets/sample/glossary-en.tsv"),
            )),
            ..AssemblySpec::new(root.join("assets/sample/dict.tsv"))
        })
        .expect("assemble engine from sample data");
        let mut router = Router::new(engine, RouterConfig::default());
        let (work_tx, work_rx) = std::sync::mpsc::channel();
        let _ = serve_pipe(&server_name, None, &mut router, work_tx, work_rx);
    });

    let mut client = connect(&name);

    write_message(
        &mut client,
        &ClientMessage::OpenSession {
            session: SESSION,
            app: None,
            protocol: PROTOCOL_VERSION,
        },
    )
    .unwrap();
    for c in "nihao".chars() {
        write_message(
            &mut client,
            &ClientMessage::Key {
                session: SESSION,
                event: letter(c),
            },
        )
        .unwrap();
    }

    // 开会话先回一条 `SessionOpened`，之后五个按键各回一条 `KeyResult`。
    let mut last_frame = None;
    for _ in 0..6 {
        let message: ServerMessage = read_message(&mut client)
            .expect("read response")
            .expect("server closed early");
        if let ServerMessage::KeyResult { frame, .. } = message {
            last_frame = Some(frame);
        }
    }

    let frame = last_frame.expect("至少一条 KeyResult");
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
