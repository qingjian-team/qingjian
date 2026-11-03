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
