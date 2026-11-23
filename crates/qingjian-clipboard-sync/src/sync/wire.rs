//! 一条 [`Record`] 的线上格式：4 字节头长度 + JSON 头 + 图片原始字节，图片不必转 base64 变大。

use serde::{Deserialize, Serialize};

use crate::error::SyncError;
use crate::record::{Record, RecordKind, Stamp};

#[derive(Serialize, Deserialize)]
struct Header {
    stamp: Stamp,

    kind: RecordKind,

    text: String,

    extension: String,
}

pub(super) fn encode_record(record: &Record) -> Result<Vec<u8>, SyncError> {
    let header = serde_json::to_vec(&Header {
        stamp: record.stamp.clone(),
        kind: record.kind,
        text: record.text.clone(),
        extension: record.extension.clone(),
    })?;
    let length = u32::try_from(header.len())
        .map_err(|_| SyncError::Protocol("record header too large".to_owned()))?;
    let mut message = Vec::with_capacity(4 + header.len() + record.image.len());
    message.extend_from_slice(&length.to_be_bytes());
    message.extend_from_slice(&header);
    message.extend_from_slice(&record.image);
    Ok(message)
}

pub(super) fn decode_record(message: &[u8]) -> Result<Record, SyncError> {
    let truncated = || SyncError::Protocol("truncated record".to_owned());
    let (length, rest) = message.split_first_chunk::<4>().ok_or_else(truncated)?;
    let length = u32::from_be_bytes(*length) as usize;
    if length > rest.len() {
        return Err(truncated());
    }
    let (header, image) = rest.split_at(length);
    let header: Header = serde_json::from_slice(header)?;
    Ok(Record {
        stamp: header.stamp,
        kind: header.kind,
        text: header.text,
        image: image.to_vec(),
        extension: header.extension,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_record(text: &str) -> Record {
        Record::text(text.to_owned(), 5)
    }

    #[test]
    fn a_text_record_round_trips() {
        let record = text_record("你好，世界");
        let message = encode_record(&record).unwrap();
        assert_eq!(decode_record(&message).unwrap(), record);
    }

    #[test]
    fn an_image_record_keeps_the_raw_bytes_at_the_end() {
        let image = vec![0x89, b'P', b'N', b'G', 0, 1, 2];
        let record = Record::image(image.clone(), "png".to_owned(), 6);
        let message = encode_record(&record).unwrap();
        assert!(message.ends_with(image.as_slice()));
        assert_eq!(decode_record(&message).unwrap(), record);
    }

    #[test]
    fn short_messages_are_truncated() {
        assert!(decode_record(b"").is_err());
        assert!(decode_record(&[0, 0, 0]).is_err());
    }

    #[test]
    fn a_header_longer_than_the_message_is_truncated() {
        let mut message = 10_u32.to_be_bytes().to_vec();
        message.extend_from_slice(b"{}");
        assert!(decode_record(&message).is_err());
    }

    #[test]
    fn a_broken_header_does_not_decode() {
        let mut message = 1_u32.to_be_bytes().to_vec();
        message.push(b'{');
        assert!(decode_record(&message).is_err());
    }

    #[test]
    fn a_tombstone_round_trips_without_content() {
        let record = Record::tombstone(text_record("x").stamp, RecordKind::Text);
        let message = encode_record(&record).unwrap();
        let decoded = decode_record(&message).unwrap();
        assert_eq!(decoded, record);
        assert!(decoded.stamp.deleted);
        assert!(decoded.text.is_empty());
    }
}
