//! 传输层测试：线上帧编解码，以及在字节流上跑完整「读消息 → Router → 写回」的 serve 循环。
//! 不碰真正的命名管道，用内存里的 Cursor / Vec 模拟一条连好的流，因而跨平台（含 Windows）可跑。

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use qingjian_core::Language;
use qingjian_platform::protocol::{ClientMessage, KeyEvent, ServerMessage, SessionId};
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
    // 流已到尽头：再读是干净 EOF。
    let end: Option<ClientMessage> = read_message(&mut reader).unwrap();
    assert_eq!(end, None);
}

#[test]
fn serve_runs_the_open_type_loop_over_a_stream() {
    // 把一串客户端消息编成输入流：开会话 + 逐键敲 nihao。
    let mut input = Vec::new();
    write_message(&mut input, &ClientMessage::OpenSession { session: SESSION }).unwrap();
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

    // 解出回复：OpenSession 不回话，5 个按键各回一条 KeyResult。
    let mut responses = Vec::new();
    let mut out = Cursor::new(stream.outbound);
    while let Some(message) = read_message::<_, ServerMessage>(&mut out).unwrap() {
        responses.push(message);
    }
    assert_eq!(responses.len(), 5, "五个按键应各回一条 KeyResult");

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

/// 把「一段预置输入 + 一个输出缓冲」拼成一条双工流，喂给 serve（半双工够测：先发后收）。
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
