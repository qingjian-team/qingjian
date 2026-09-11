//! 命名管道传输的端到端测试（仅 Windows）：起一个监听线程，客户端用文件句柄连上同名管道，
//! 走完「开会话 → 敲 nihao → 收候选」。验证真正的进程间管道（不是内存流）在 Windows 上通。

#![cfg(windows)]

use std::fs::OpenOptions;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use qingjian_core::Language;
use qingjian_platform::protocol::{ClientMessage, KeyEvent, ServerMessage, SessionId};
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

#[test]
fn named_pipe_round_trips_the_open_type_loop() {
    // 每个测试进程一个独占管道名，避免撞车。
    let name = format!(r"\\.\pipe\qingjian-test-{}", std::process::id());

    // 监听线程：装配 Engine 并在管道上服务（服务完一个客户端后会阻塞等下一个，随进程退出即可）。
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
        let _ = serve_pipe(&server_name, &mut router);
    });

    let mut client = connect(&name);

    // 开会话（不回话）+ 逐键敲 nihao（各回一条 KeyResult）。
    write_message(
        &mut client,
        &ClientMessage::OpenSession {
            session: SESSION,
            app: None,
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

    let mut last_frame = None;
    for _ in 0..5 {
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
