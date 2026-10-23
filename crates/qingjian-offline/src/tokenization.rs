//! ONNX 导出包的 charsmap 是空占位符，使用同一版本 source.spm 的实际归一化表。

use std::path::Path;

use tokenizers::Tokenizer;
use tokenizers::normalizers::Precompiled;

use crate::OfflineError;

pub(super) fn load(dir: &Path) -> Result<Tokenizer, OfflineError> {
    let mut json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("tokenizer.json"))?)?;
    // tokenizers 对 null charsmap 会 panic；不能把这个占位符交给它反序列化。
    json["normalizer"] = serde_json::Value::Null;
    let mut tokenizer = Tokenizer::from_bytes(serde_json::to_vec(&json)?)
        .map_err(|e| OfflineError::Inference(e.to_string()))?;
    let spm = std::fs::read(dir.join("source.spm"))?;
    let normalizer = bytes_field(&spm, 3)?.ok_or(OfflineError::Invalid("missing normalizer"))?;
    let map = bytes_field(normalizer, 2)?.ok_or(OfflineError::Invalid("missing charsmap"))?;
    let precompiled = Precompiled::from(map).map_err(|e| OfflineError::Inference(e.to_string()))?;
    tokenizer.with_normalizer(Some(precompiled));
    Ok(tokenizer)
}

// SentencePiece ModelProto 的 3 号字段是 NormalizerSpec，后者的 2 号字段是 charsmap。
// 只读这两个 length-delimited 字段，其他字段按 Protobuf wire type 跳过。
fn bytes_field(mut data: &[u8], wanted: u64) -> Result<Option<&[u8]>, OfflineError> {
    while !data.is_empty() {
        let key = varint(&mut data)?;
        let count = match key & 7 {
            0 => {
                varint(&mut data)?;
                continue;
            }
            1 => 8,
            2 => usize::try_from(varint(&mut data)?)
                .map_err(|_| OfflineError::Invalid("invalid SPM field size"))?,
            5 => 4,
            _ => return Err(OfflineError::Invalid("invalid SPM wire type")),
        };
        if count > data.len() {
            return Err(OfflineError::Invalid("truncated SPM field"));
        }
        let (field, remaining) = data.split_at(count);
        if key & 7 == 2 && key >> 3 == wanted {
            return Ok(Some(field));
        }
        data = remaining;
    }
    Ok(None)
}

fn varint(data: &mut &[u8]) -> Result<u64, OfflineError> {
    let mut value = 0;
    for shift in (0..70).step_by(7) {
        let (&byte, remaining) = data
            .split_first()
            .ok_or(OfflineError::Invalid("truncated SPM varint"))?;
        *data = remaining;
        if shift == 63 && byte > 1 {
            return Err(OfflineError::Invalid("SPM varint overflow"));
        }
        value |= u64::from(byte & 127) << shift;
        if byte < 128 {
            return Ok(value);
        }
    }
    Err(OfflineError::Invalid("SPM varint overflow"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn malformed_fields_return_errors_instead_of_panicking() {
        assert!(super::bytes_field(&[0x1a, 10, 1], 3).is_err());
        assert!(super::bytes_field(&[0x1a, 0xff], 3).is_err());
        assert!(super::bytes_field(&[0x1f], 3).is_err());
        assert!(super::bytes_field(&[0xff; 12], 3).is_err());
    }

    #[test]
    fn skips_other_fields_and_reads_only_the_requested_bytes() {
        assert_eq!(
            super::bytes_field(&[8, 1, 0x1a, 2, 3, 4], 3).unwrap(),
            Some(&[3, 4][..])
        );
    }
}
