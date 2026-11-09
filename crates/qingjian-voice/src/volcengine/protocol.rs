//! v3 二进制帧：4 字节头（版本 / 头长、消息类型 / 标志、序列化 / 压缩、保留），按标志带 4 字节序号，再是 4 字节长度与载荷；整数大端。
//! 发出去的载荷一律 gzip。

use std::io::{Read, Write};

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;

use crate::VoiceError;

const VERSION_AND_HEADER_SIZE: u8 = 0x11;

const FULL_CLIENT_REQUEST: u8 = 0b0001;
const AUDIO_ONLY_REQUEST: u8 = 0b0010;
const FULL_SERVER_RESPONSE: u8 = 0b1001;
const SERVER_ERROR: u8 = 0b1111;

const POSITIVE_SEQUENCE: u8 = 0b0001;
const LAST_PACKET: u8 = 0b0010;

const NO_SERIALIZATION: u8 = 0b0000;
const JSON: u8 = 0b0001;
const GZIP: u8 = 0b0001;

/// 服务端发来的一帧。
#[derive(Debug, PartialEq, Eq)]
pub enum Incoming {
    /// 识别结果（JSON，已解压）；`last` 是这段的最后一包。
    Response {
        last: bool,
        payload: Vec<u8>,
    },

    Error {
        code: u32,
        message: String,
    },

    /// 别的类型（确认包之类），不用管。
    Other,
}

/// 开头那包：识别参数的 JSON，序号 1。
pub fn full_request(json: &[u8]) -> Vec<u8> {
    frame(FULL_CLIENT_REQUEST, POSITIVE_SEQUENCE, JSON, 1, json)
}

/// 一包音频（小端 16 位 PCM）；最后一包序号取负。
pub fn audio(sequence: i32, pcm: &[u8], last: bool) -> Vec<u8> {
    let (flags, sequence) = if last {
        (POSITIVE_SEQUENCE | LAST_PACKET, -sequence)
    } else {
        (POSITIVE_SEQUENCE, sequence)
    };
    frame(AUDIO_ONLY_REQUEST, flags, NO_SERIALIZATION, sequence, pcm)
}

fn frame(kind: u8, flags: u8, serialization: u8, sequence: i32, payload: &[u8]) -> Vec<u8> {
    let payload = gzip(payload);
    let mut frame = Vec::with_capacity(12 + payload.len());
    frame.extend_from_slice(&[
        VERSION_AND_HEADER_SIZE,
        (kind << 4) | flags,
        (serialization << 4) | GZIP,
        0,
    ]);
    frame.extend_from_slice(&sequence.to_be_bytes());
    frame.extend_from_slice(
        &u32::try_from(payload.len())
            .unwrap_or(u32::MAX)
            .to_be_bytes(),
    );
    frame.extend_from_slice(&payload);
    frame
}

pub fn parse(frame: &[u8]) -> Result<Incoming, VoiceError> {
    let header = *frame
        .first_chunk::<4>()
        .ok_or(VoiceError::Protocol("short header"))?;
    let header_size = usize::from(header[0] & 0x0f) * 4;
    let kind = header[1] >> 4;
    let flags = header[1] & 0x0f;
    let compressed = header[2] & 0x0f == GZIP;
    let mut rest = frame
        .get(header_size..)
        .ok_or(VoiceError::Protocol("short header"))?;
    if flags & POSITIVE_SEQUENCE != 0 {
        rest = rest
            .get(4..)
            .ok_or(VoiceError::Protocol("missing sequence"))?;
    }
    match kind {
        FULL_SERVER_RESPONSE => {
            let payload = sized(rest)?;
            Ok(Incoming::Response {
                last: flags & LAST_PACKET != 0,
                payload: unpack(payload, compressed)?,
            })
        }
        SERVER_ERROR => {
            let code = u32::from_be_bytes(
                *rest
                    .first_chunk::<4>()
                    .ok_or(VoiceError::Protocol("missing error code"))?,
            );
            let message = unpack(sized(&rest[4..])?, compressed)?;
            Ok(Incoming::Error {
                code,
                message: String::from_utf8_lossy(&message).into_owned(),
            })
        }
        _ => Ok(Incoming::Other),
    }
}

/// 4 字节长度加那么长的载荷。
fn sized(bytes: &[u8]) -> Result<&[u8], VoiceError> {
    let size = u32::from_be_bytes(
        *bytes
            .first_chunk::<4>()
            .ok_or(VoiceError::Protocol("missing size"))?,
    );
    let size = usize::try_from(size).map_err(|_| VoiceError::Protocol("bad size"))?;
    bytes
        .get(4..4 + size)
        .ok_or(VoiceError::Protocol("truncated payload"))
}

fn unpack(payload: &[u8], compressed: bool) -> Result<Vec<u8>, VoiceError> {
    if !compressed {
        return Ok(payload.to_vec());
    }
    let mut out = Vec::new();
    GzDecoder::new(payload)
        .read_to_end(&mut out)
        .map_err(|_| VoiceError::Protocol("bad gzip"))?;
    Ok(out)
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    // 写进内存缓冲不会失败
    let _ = encoder.write_all(bytes);
    encoder.finish().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 拼一帧服务端回包：同发出去的格式，类型与标志按参数。
    fn server_frame(kind: u8, flags: u8, body: &[u8]) -> Vec<u8> {
        let payload = gzip(body);
        let mut frame = vec![
            VERSION_AND_HEADER_SIZE,
            (kind << 4) | flags,
            (JSON << 4) | GZIP,
            0,
        ];
        if flags & POSITIVE_SEQUENCE != 0 {
            frame.extend_from_slice(&3i32.to_be_bytes());
        }
        frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        frame.extend_from_slice(&payload);
        frame
    }

    #[test]
    fn request_header_and_payload() {
        let frame = full_request(b"{}");
        assert_eq!(&frame[..4], &[0x11, 0x11, 0x11, 0x00]);
        assert_eq!(&frame[4..8], &1i32.to_be_bytes());
        let size = u32::from_be_bytes(frame[8..12].try_into().unwrap()) as usize;
        assert_eq!(unpack(&frame[12..12 + size], true).unwrap(), b"{}");
    }

    #[test]
    fn last_audio_packet_negates_sequence() {
        let frame = audio(5, &[1, 0, 2, 0], true);
        assert_eq!(frame[1], (AUDIO_ONLY_REQUEST << 4) | 0b0011);
        assert_eq!(frame[2], GZIP);
        assert_eq!(&frame[4..8], &(-5i32).to_be_bytes());
    }

    #[test]
    fn parses_response_with_sequence() {
        let frame = server_frame(FULL_SERVER_RESPONSE, 0b0011, br#"{"result":{"text":"hi"}}"#);
        assert_eq!(
            parse(&frame).unwrap(),
            Incoming::Response {
                last: true,
                payload: br#"{"result":{"text":"hi"}}"#.to_vec()
            }
        );
        let frame = server_frame(FULL_SERVER_RESPONSE, 0b0001, b"{}");
        assert!(matches!(
            parse(&frame).unwrap(),
            Incoming::Response { last: false, .. }
        ));
    }

    #[test]
    fn parses_error() {
        let message = gzip(b"bad audio");
        let mut frame = vec![0x11, SERVER_ERROR << 4, (JSON << 4) | GZIP, 0];
        frame.extend_from_slice(&45_000_151u32.to_be_bytes());
        frame.extend_from_slice(&(message.len() as u32).to_be_bytes());
        frame.extend_from_slice(&message);
        assert_eq!(
            parse(&frame).unwrap(),
            Incoming::Error {
                code: 45_000_151,
                message: "bad audio".to_owned()
            }
        );
    }

    #[test]
    fn rejects_truncated() {
        assert!(parse(&[0x11]).is_err());
        let mut frame = server_frame(FULL_SERVER_RESPONSE, 0, b"{}");
        frame.truncate(frame.len() - 1);
        assert!(parse(&frame).is_err());
    }
}
