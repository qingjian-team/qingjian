//! 损坏 / 缺失 / 空词库等错误路径。

use super::*;

use qingjian_format::{FormatError, Kind, Metadata, Writer};

use crate::error::DictionaryError;
use crate::pattern::SyllablePattern;

#[test]
fn rejects_truncated_qj_file() {
    let dictionary = Dictionary::parse(SAMPLE).unwrap();
    let path = temp_path("truncated");
    dictionary.write_qj(&path, &Metadata::default()).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    std::fs::write(&path, &bytes[..bytes.len() / 2]).unwrap();
    let result = Dictionary::open_qj(&path);
    std::fs::remove_file(&path).unwrap();
    assert!(result.is_err(), "truncated file must fail to open");
}

#[test]
fn rejects_bad_magic() {
    let path = temp_path("bad-magic");
    std::fs::write(
        &path,
        b"not a qj file at all, definitely longer than a header",
    )
    .unwrap();
    let result = Dictionary::open_qj(&path);
    std::fs::remove_file(&path).unwrap();
    assert!(matches!(
        result,
        Err(DictionaryError::Format(FormatError::BadMagic))
    ));
}

#[test]
fn rejects_unsupported_version() {
    let dictionary = Dictionary::parse(SAMPLE).unwrap();
    let path = temp_path("bad-version");
    dictionary.write_qj(&path, &Metadata::default()).unwrap();
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[8..10].copy_from_slice(&999u16.to_le_bytes());
    std::fs::write(&path, &bytes).unwrap();
    let result = Dictionary::open_qj(&path);
    std::fs::remove_file(&path).unwrap();
    assert!(matches!(
        result,
        Err(DictionaryError::Format(FormatError::UnsupportedVersion(
            999
        )))
    ));
}

#[test]
fn rejects_zero_byte_file() {
    let path = temp_path("zero-byte");
    std::fs::write(&path, [] as [u8; 0]).unwrap();
    let result = Dictionary::open_qj(&path);
    std::fs::remove_file(&path).unwrap();
    assert!(result.is_err());
}

#[test]
fn missing_file_is_reported_as_io_error() {
    let path = temp_path("missing");
    let _ = std::fs::remove_file(&path);
    assert!(Dictionary::from_path(&path).is_err());
}

#[test]
fn rejects_slot_pointing_outside_the_text_arena() {
    let dictionary = Dictionary::parse(SAMPLE).unwrap();
    let path = temp_path("corrupt-slot");
    dictionary.write_qj(&path, &Metadata::default()).unwrap();
    let mut bytes = std::fs::read(&path).unwrap();
    let count = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let mut slots_offset = None;
    for k in 0..count {
        let base = 32 + k * 24;
        if &bytes[base..base + 4] == b"SLOT" {
            slots_offset =
                Some(u64::from_le_bytes(bytes[base + 8..base + 16].try_into().unwrap()) as usize);
        }
    }
    let slots_offset = slots_offset.expect("SLOTS section present");
    bytes[slots_offset..slots_offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    std::fs::write(&path, &bytes).unwrap();
    let result = Dictionary::open_qj(&path);
    std::fs::remove_file(&path).unwrap();
    assert!(matches!(result, Err(DictionaryError::Corrupt(_))));
}

#[test]
fn rejects_wrong_data_kind() {
    let dir = std::env::temp_dir().join("qingjian-dictionary-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("wrong-kind-{}.qj", std::process::id()));
    Writer::new(Kind::CodeTable, &Metadata::default())
        .unwrap()
        .section(*b"TEXT", b"x")
        .write_to(&path)
        .unwrap();
    let result = Dictionary::open_qj(&path);
    std::fs::remove_file(&path).unwrap();
    assert!(matches!(
        result,
        Err(DictionaryError::Format(FormatError::WrongKind { .. }))
    ));
}

#[test]
fn empty_dictionary_queries_are_empty() {
    let dictionary = Dictionary::parse("").unwrap();
    assert!(dictionary.is_empty());
    assert_eq!(dictionary.len(), 0);
    assert_eq!(dictionary.total_frequency(), 0);
    assert!(dictionary.entries().next().is_none());
    assert!(dictionary.lookup(&["kai"], false).is_empty());
    assert!(
        dictionary
            .lookup_pattern(&[SyllablePattern::prefix("k")])
            .is_empty()
    );
}
