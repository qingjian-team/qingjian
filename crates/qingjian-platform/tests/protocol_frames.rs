//! 线上帧的畸形输入。
//!
//! 这是 Server 与 DLL 之间唯一的外部输入边界：命名管道里过来的任何字节都可能是坏的
//! （另一个进程崩在写帧中间、DLL 版本对不上、有人拿工具往管道里灌）。
//! `read_message` 的三道防线——`MAX_FRAME` 挡住坏长度前缀导致的巨量分配、
//! `UnexpectedEof` 区分「读到一半断了」与「帧边界干净关闭」、JSON 错误兜住语法与类型——
//! 以前一个测试都没有，`tests/pipe.rs` 只验了一趟正常往返。
//!
//! 关注点是**行为**不是实现：每条都断言返回哪个错误、以及没有 panic 或巨量分配。
//! 尤其那条逐字节喂的用例：`Read::read` 允许短读，`read_fully_or_eof` 的循环必须扛住，
//! 而 `Cursor` 的默认实现总是一次读满，用它测不到。

use std::io::{self, Cursor, Read};

use qingjian_platform::protocol::{
    ClientMessage, CodecError, Frame, KeyOutcome, ServerMessage, SessionId, read_message,
    write_message,
};

/// 帧上限，与 codec 里的常量一致（16 MiB）。写成字面量是故意的：常量改了这里会红，
/// 那时要重新想一个多大的上限才够，而不是让两边悄悄漂开。
const MAX_FRAME: u32 = 16 * 1024 * 1024;

/// `Read` 每次最多给 `chunk` 字节，用来逼出短读路径。
struct Trickle {
    data: Vec<u8>,
    pos: usize,
    chunk: usize,
}

impl Read for Trickle {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.data.len() || buf.is_empty() {
            return Ok(0);
        }
        let n = buf.len().min(self.chunk).min(self.data.len() - self.pos);
        buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

fn frame_of(body: &[u8]) -> Vec<u8> {
    let mut out = (body.len() as u32).to_le_bytes().to_vec();
    out.extend_from_slice(body);
    out
}

fn err_string(e: CodecError) -> String {
    e.to_string()
}

/// 一个够用的样本消息，长度前缀按真帧算。
fn sample_bytes() -> Vec<u8> {
    let msg = ServerMessage::KeyResult {
        session: SessionId(7),
        outcome: KeyOutcome::Consumed,
        commit: None,
        frame: Frame::default(),
    };
    let mut buf = Vec::new();
    write_message(&mut buf, &msg).expect("写样本帧");
    buf
}

#[test]
fn a_clean_round_trip_still_works() {
    // 对照组：前面那些用例都是在坏输入上兜住，这条守住「没把好路径也一起兜没了」。
    let bytes = sample_bytes();
    let mut cursor = Cursor::new(bytes);
    let got: Option<ServerMessage> = read_message(&mut cursor).expect("读样本帧");
    match got {
        Some(ServerMessage::KeyResult { session, .. }) => assert_eq!(session, SessionId(7)),
        other => panic!("要 KeyResult，实际 {other:?}"),
    }
}

#[test]
fn an_empty_stream_is_a_clean_close_not_an_error() {
    // 对端在帧边界关闭（读长度前缀时就是 EOF）必须是 Ok(None)：Server 靠这个区分
    // 「应用正常退出了」与「连接坏了」，两者处理方式不同。
    let mut cursor = Cursor::new(Vec::new());
    let got: Option<ServerMessage> = read_message(&mut cursor).expect("空流不该报错");
    assert!(got.is_none(), "空流应读成 None，实际 {got:?}");
}

#[test]
fn a_frame_cut_in_half_is_unexpected_eof() {
    let bytes = sample_bytes();
    let cut = &bytes[..bytes.len() - 5];
    let mut cursor = Cursor::new(cut.to_vec());
    let e = read_message::<_, ServerMessage>(&mut cursor).expect_err("半截帧该报错");
    assert!(
        matches!(e, CodecError::Io(ref io) if io.kind() == io::ErrorKind::UnexpectedEof),
        "要 UnexpectedEof，实际 {}",
        err_string(e)
    );
}

#[test]
fn a_length_prefix_past_the_body_is_unexpected_eof() {
    // 长度前缀说 200 字节、实际只给了 10 字节：必须挂在 UnexpectedEof 上，
    // 不能读成 Ok(Some(...)) 让上层拿半截消息当完整的用。
    let mut bytes = (200u32).to_le_bytes().to_vec();
    bytes.extend_from_slice(b"{\"abc\":1}");
    let mut cursor = Cursor::new(bytes);
    let e = read_message::<_, ServerMessage>(&mut cursor).expect_err("长度对不上该报错");
    assert!(
        matches!(e, CodecError::Io(ref io) if io.kind() == io::ErrorKind::UnexpectedEof),
        "要 UnexpectedEof，实际 {}",
        err_string(e)
    );
}

#[test]
fn an_oversized_length_prefix_is_rejected_before_allocating() {
    // 这条是 MAX_FRAME 存在的全部理由：一个坏掉的长度前缀如果被照单分配，
    // 一个字节就能让 Server 去申请 4 GiB。所以要连 `body` 都不去读就报错——
    // 用「后面什么都不给」构造：只发前缀就 EOF，若代码先分配再读，
    // 这里会先撞 UnexpectedEof，错误种类就分不出来了。
    let mut bytes = (MAX_FRAME + 1).to_le_bytes().to_vec();
    bytes.extend_from_slice(b"{\"pad\":\"");
    let mut cursor = Cursor::new(bytes);
    let e = read_message::<_, ServerMessage>(&mut cursor).expect_err("超长前缀该报错");
    assert!(
        matches!(e, CodecError::TooLarge(n) if n == MAX_FRAME as usize + 1),
        "要 TooLarge(MAX_FRAME+1)，实际 {}",
        err_string(e)
    );
}

#[test]
fn the_largest_allowed_frame_is_not_rejected_for_its_size() {
    // 上限的另一侧：恰好等于 MAX_FRAME 的长度前缀不该被「超长」这条拦下，
    // 否则上限就成了「比它小的合法帧」。这里只验前缀判定，body 补零 JSON 非法是下一步的事。
    let mut bytes = MAX_FRAME.to_le_bytes().to_vec();
    bytes.truncate(4); // body 一个字节都不给 → 撞 UnexpectedEof 而不是 TooLarge
    let mut cursor = Cursor::new(bytes);
    let e = read_message::<_, ServerMessage>(&mut cursor).expect_err("body 缺了该报错");
    assert!(
        matches!(e, CodecError::Io(ref io) if io.kind() == io::ErrorKind::UnexpectedEof),
        "MAX_FRAME 本身不该被当超长拦下，实际 {}",
        err_string(e)
    );
}

#[test]
fn a_zero_length_frame_is_a_json_error() {
    // len = 0 不是「空帧」，是「0 字节的 JSON」。serde_json 会报 EOF 而不是 Ok(None)——
    // 这条把现状钉住，免得有人日后顺手改成「len=0 视作关闭」。
    let bytes = frame_of(b"");
    let mut cursor = Cursor::new(bytes);
    let e = read_message::<_, ServerMessage>(&mut cursor).expect_err("零长帧不是合法 JSON");
    assert!(
        matches!(e, CodecError::Json(_)),
        "要 Json 错误，实际 {}",
        err_string(e)
    );
}

#[test]
fn garbage_bytes_come_back_as_a_json_error() {
    for body in [
        &b"\xff\xfe\x00\x01"[..],                    // 非 UTF-8
        &b"not json at all"[..],                     // 纯垃圾
        &b"{\"session\":1"[..],                      // 截断的 JSON
        &b"[1,2,3]"[..],                             // 类型不对：数组不是消息
        &b"null"[..],                                // 能解析但不是消息
        &b"{\"__proto__\":1,\"__unknown__\":2}"[..], // 未知变体名
    ] {
        let bytes = frame_of(body);
        let mut cursor = Cursor::new(bytes);
        let e = read_message::<_, ServerMessage>(&mut cursor)
            .expect_err(&format!("{body:?} 不该读成消息"));
        assert!(
            matches!(e, CodecError::Json(_)),
            "{body:?} 要 Json 错误，实际 {}",
            err_string(e)
        );
    }
}

#[test]
fn a_client_message_decodes_the_same_way() {
    // 方向反过来也验一次：写 ClientMessage、读 ClientMessage。两端共用 codec.rs，
    // 只验一个方向的话，另一半的泛型推导错了也不会有人发现。
    let msg = ClientMessage::HideCandidates {
        session: SessionId(3),
    };
    let mut buf = Vec::new();
    write_message(&mut buf, &msg).expect("写 ClientMessage");
    let mut cursor = Cursor::new(buf);
    let got: Option<ClientMessage> = read_message(&mut cursor).expect("读 ClientMessage");
    assert_eq!(got, Some(msg), "往返后应与原消息相等");
}

#[test]
fn frames_arriving_one_byte_at_a_time_still_parse() {
    // `Read::read` 允许短读。命名管道通常一次给全，但 split/合并、代理转发、
    // 或者别的 `Read` 实现都可能只给几个字节——`read_fully_or_eof` 的循环必须扛住。
    // 用 `Cursor` 测不到这条：它的默认实现总是一次读满请求长度。
    let bytes = sample_bytes();
    let mut trickle = Trickle {
        data: bytes.clone(),
        pos: 0,
        chunk: 1,
    };
    let got: Option<ServerMessage> = read_message(&mut trickle).expect("逐字节也要读得出来");
    assert!(
        matches!(got, Some(ServerMessage::KeyResult { .. })),
        "实际 {got:?}"
    );
}

#[test]
fn two_frames_in_one_read_come_back_one_at_a_time() {
    // 对面连续发两帧（按键连发就是），一次 read 拿到的字节流里有两帧。
    // 少读一帧会让 Server 与 DLL 的帧边界错位，之后每一帧都变成半截的。
    let mut bytes = sample_bytes();
    bytes.extend_from_slice(&sample_bytes());
    let mut cursor = Cursor::new(bytes);
    let first: Option<ServerMessage> = read_message(&mut cursor).expect("第一帧");
    let second: Option<ServerMessage> = read_message(&mut cursor).expect("第二帧");
    assert!(first.is_some(), "第一帧应有内容");
    assert!(
        matches!(second, Some(ServerMessage::KeyResult { .. })),
        "第二帧应有内容，实际 {second:?}"
    );
}

#[test]
fn a_wrong_length_prefix_that_looks_like_valid_json_still_needs_the_rest() {
    // 攻击面具体化：前一次会话留下的字节（合法 JSON、合法长度前缀）被接上，
    // 读出来是一帧语法完整但语义不对的消息。这是长度前缀不校验内容时
    // 无法自己发现的形状——它只能靠「长度前缀与 body 对得上」来保证帧边界，
    // 也就是上面那几条（截断 / 超长）真正在守的东西。
    let mut bytes = frame_of(br#"{"session":7}"#);
    bytes.extend_from_slice(&sample_bytes());
    let mut cursor = Cursor::new(bytes);
    // 第一帧的 JSON 缺必填字段，serde 会报错，读到第二帧的字节之前就停。
    let e = read_message::<_, ServerMessage>(&mut cursor).expect_err("缺字段该报错");
    assert!(
        matches!(e, CodecError::Json(_)),
        "要 Json 错误，实际 {}",
        err_string(e)
    );
}
