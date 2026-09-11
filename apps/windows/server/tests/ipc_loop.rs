//! 传输层测试：帧编解码，以及在内存流上跑 serve 循环；不碰命名管道，跨平台可跑。

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use qingjian_core::Language;
use qingjian_platform::protocol::{
    ClientMessage, KeyEvent, PROTOCOL_VERSION, ServerMessage, SessionId,
};
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
