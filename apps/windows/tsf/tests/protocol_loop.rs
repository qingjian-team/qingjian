//! DLL 引擎层的端到端协议测试：把 [`EngineClient`] 接到真正的 Server（`qingjian-windows-server` 的
//! [`Router`] + [`serve`](qingjian_windows_server::ipc::serve)），两端各在一条 socketpair 上，验证
//! 「开会话 → 敲拼音收到候选 → 空格上屏」这条 IPC 闭环。
//!
//! 用 `UnixStream::pair` 起真双工流，所以只在 Unix 跑（mac 上开发时能验证 client 编排）；Windows 上
//! 同一套 [`EngineClient`] 由命名管道驱动，靠交互测试。样例词库来自 `assets/sample/`，无需产品数据。
#![cfg(unix)]

use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::thread;

use qingjian_core::Language;
use qingjian_platform::protocol::{KeyEvent, KeyOutcome, SessionId};
use qingjian_tsf::client::EngineClient;
use qingjian_windows_server::{AssemblySpec, Router, RouterConfig, assembly, ipc};

const SESSION: SessionId = SessionId(1);

/// 一个字母键（`character` 带小写字母，虚拟键码用其大写 ASCII）。
fn letter(c: char) -> KeyEvent {
    KeyEvent::new(c.to_ascii_uppercase() as u32, Some(c), Default::default())
}

/// 起一个后台 Server：用样例词库装 Router，在 `server_end` 上 serve 到对端关闭。
fn spawn_server(server_end: UnixStream) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let dict = root.join("assets/sample/dict.tsv");
        let glossary = root.join("assets/sample/glossary-en.tsv");
        let engine = assembly::assemble(&AssemblySpec {
            glossary: Some((Language::English, glossary)),
            ..AssemblySpec::new(dict)
        })
        .expect("assemble engine from sample data");
        let mut router = Router::new(engine, RouterConfig::default());
        let mut stream = server_end;
        let _ = ipc::serve(&mut stream, &mut router);
    })
}

#[test]
fn client_types_pinyin_and_gets_candidates() {
    let (client_end, server_end) = UnixStream::pair().unwrap();
    let server = spawn_server(server_end);

    let mut client = EngineClient::open(client_end, SESSION, None).expect("open session");
    let mut last = None;
    for c in "nihao".chars() {
        last = Some(client.key(letter(c)).expect("key round-trips"));
    }
    let response = last.unwrap();

    assert_eq!(response.outcome, KeyOutcome::Consumed);
    assert_eq!(response.commit, None);
    let preedit: String = response
        .frame
        .preedit
        .iter()
        .map(|s| s.text.as_str())
        .collect();
    assert_eq!(preedit, "ni'hao");
    let texts: Vec<&str> = response
        .frame
        .candidates
        .items
        .iter()
        .map(|c| c.text.as_str())
        .collect();
    assert!(
        texts.contains(&"你好"),
        "候选里应有「你好」，实际：{texts:?}"
    );

    client.close().expect("close session");
    server.join().unwrap();
}

#[test]
fn space_commits_first_candidate() {
    let (client_end, server_end) = UnixStream::pair().unwrap();
    let server = spawn_server(server_end);

    let mut client = EngineClient::open(client_end, SESSION, None).expect("open session");
    for c in "ni".chars() {
        client.key(letter(c)).expect("key round-trips");
    }
    let space = client
        .key(KeyEvent::new(0x20, Some(' '), Default::default()))
        .expect("space round-trips");

    assert_eq!(space.outcome, KeyOutcome::Consumed);
    assert_eq!(space.commit.as_deref(), Some("你"), "「ni」首选应是「你」");
    assert!(space.frame.is_empty(), "上屏后应收起候选");

    client.close().expect("close session");
    server.join().unwrap();
}

#[test]
fn commit_returns_raw_text() {
    let (client_end, server_end) = UnixStream::pair().unwrap();
    let server = spawn_server(server_end);

    let mut client = EngineClient::open(client_end, SESSION, None).expect("open session");
    for c in "nihao".chars() {
        client.key(letter(c)).expect("key round-trips");
    }
    let text = client.commit().expect("commit round-trips");
    assert_eq!(text.as_deref(), Some("nihao"), "失焦时拼音原样交出");
    assert_eq!(
        client.commit().expect("commit round-trips"),
        None,
        "缓冲已清空"
    );

    client.close().expect("close session");
    server.join().unwrap();
}
