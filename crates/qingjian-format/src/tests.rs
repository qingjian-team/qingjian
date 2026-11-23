//! 容器格式的常量、布局与错误文案；打开/写入流程在 `file/` 与 `qjz/` 的测试里。

use std::io;

use zerocopy::IntoBytes;

use crate::{
    EXTENSION, FORMAT_VERSION, FormatError, Header, Kind, MAGIC, META_TAG, Metadata, SECTION_ALIGN,
    SectionEntry, Table, Text, tag_str,
};

#[test]
fn format_constants_stay_put() {
    assert_eq!(&MAGIC, b"QINGJIAN");
    assert_eq!(FORMAT_VERSION, 1);
    assert_eq!(EXTENSION, "qj");
    assert_eq!(SECTION_ALIGN, 8);
    assert_eq!(&META_TAG, b"META");
}

#[test]
fn header_is_thirty_two_bytes() {
    assert_eq!(Header::SIZE, 32);
    let header = Header {
        magic: MAGIC,
        version: FORMAT_VERSION,
        kind: Kind::Dictionary.code(),
        section_count: 3,
        reserved: [0; 16],
    };
    let bytes = header.as_bytes();
    assert_eq!(bytes.len(), 32);
    assert_eq!(&bytes[..8], &MAGIC);
    assert_eq!(&bytes[8..10], &FORMAT_VERSION.to_le_bytes());
    assert_eq!(&bytes[10..12], &Kind::Dictionary.code().to_le_bytes());
    assert_eq!(&bytes[12..16], &3u32.to_le_bytes());
    assert_eq!(&bytes[16..], &[0; 16]);
}

#[test]
fn section_entry_is_twenty_four_bytes() {
    assert_eq!(SectionEntry::SIZE, 24);
    let entry = SectionEntry {
        tag: *b"TEXT",
        reserved: 0,
        offset: 4096,
        length: 128,
    };
    let bytes = entry.as_bytes();
    assert_eq!(&bytes[..4], b"TEXT");
    assert_eq!(&bytes[8..16], &4096u64.to_le_bytes());
    assert_eq!(&bytes[16..24], &128u64.to_le_bytes());
}

#[test]
fn every_kind_round_trips_through_its_code() {
    for kind in [
        Kind::Dictionary,
        Kind::LanguageModel,
        Kind::Glossary,
        Kind::Emoji,
        Kind::WordList,
        Kind::Model,
        Kind::AuxCodeTable,
        Kind::CodeTable,
    ] {
        assert_eq!(Kind::from_code(kind.code()), Some(kind));
    }
    assert_eq!(Kind::Dictionary.code(), 1);
    assert_eq!(Kind::CodeTable.code(), 8);
    assert_eq!(Kind::from_code(0), None);
    assert_eq!(Kind::from_code(9), None);
    assert_eq!(Kind::from_code(u16::MAX), None);
}

#[test]
fn metadata_round_trips_through_toml() {
    let metadata = Metadata {
        name: "青简基础词库".to_owned(),
        license: "CC-BY-SA-4.0".to_owned(),
        attribution: "青简".to_owned(),
        source: "https://example.com".to_owned(),
        version: "2026-09".to_owned(),
        description: "日常词".to_owned(),
        entries: 12_345,
        generator: "qingjian-dict-convert 0.1.0".to_owned(),
    };
    let text = metadata.to_toml().unwrap();
    assert_eq!(Metadata::from_toml(&text).unwrap(), metadata);
}

#[test]
fn empty_description_is_left_out_of_the_file() {
    let metadata = Metadata {
        name: "基础".to_owned(),
        ..Metadata::default()
    };
    assert!(!metadata.to_toml().unwrap().contains("description"));
}

#[test]
fn missing_fields_in_toml_fall_back_to_defaults() {
    let metadata = Metadata::from_toml("name = \"基础\"\n").unwrap();
    assert_eq!(metadata.name, "基础");
    assert_eq!(metadata.license, "");
    assert_eq!(metadata.attribution, "");
    assert_eq!(metadata.entries, 0);
    assert_eq!(metadata.description, "");
}

#[test]
fn broken_toml_is_an_error() {
    assert!(Metadata::from_toml("name = [").is_err());
}

#[test]
fn tag_str_shows_graphic_bytes_and_question_marks() {
    assert_eq!(tag_str(b"META"), "META");
    assert_eq!(tag_str(&[b'A', 0, b'B', 0x7f]), "A?B?");
    assert_eq!(tag_str(&[0x80, b'x', 0x1f, b'y']), "?x?y");
}

#[test]
fn error_messages_name_the_problem() {
    assert!(FormatError::BadMagic.to_string().contains("bad magic"));
    let version = FormatError::UnsupportedVersion(2).to_string();
    assert!(version.contains("unsupported"), "{version}");
    assert!(version.contains('2'), "{version}");
    assert!(FormatError::Truncated.to_string().contains("truncated"));
    assert_eq!(
        FormatError::MissingSection(*b"TEXT").to_string(),
        "missing section TEXT"
    );
    assert_eq!(
        FormatError::Malformed {
            tag: *b"ENTR",
            reason: "bad offsets",
        }
        .to_string(),
        "malformed section ENTR: bad offsets"
    );
    assert!(
        FormatError::SectionTooLarge(*b"TEXT")
            .to_string()
            .contains("too large")
    );
    assert!(
        FormatError::UnknownCompression
            .to_string()
            .contains("unknown compression")
    );
    assert!(FormatError::TooLarge.to_string().contains("allowed size"));
    assert!(
        FormatError::WrongKind {
            expected: Kind::Model,
            found: 9,
        }
        .to_string()
        .contains("wrong data kind")
    );
}

#[test]
fn metadata_and_io_errors_convert_into_format_errors() {
    let error = FormatError::from(toml::from_str::<Metadata>("name = [").unwrap_err());
    assert!(error.to_string().contains("TOML"));
    let error = FormatError::from(io::Error::new(io::ErrorKind::NotFound, "missing"));
    assert!(error.to_string().contains("missing"));
}

#[test]
fn owned_text_derefs_to_str() {
    let text = Text::Owned("词库".to_owned());
    assert_eq!(&*text, "词库");
    assert!(Text::default().is_empty());
}

#[test]
fn owned_tables_deref_and_serialize() {
    let entries = Table::Owned(vec![
        SectionEntry {
            tag: *b"META",
            reserved: 0,
            offset: 32,
            length: 64,
        },
        SectionEntry {
            tag: *b"TEXT",
            reserved: 0,
            offset: 96,
            length: 8,
        },
    ]);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[1].tag, *b"TEXT");
    assert_eq!(entries.as_bytes().len(), 2 * SectionEntry::SIZE);
    assert!(Table::<SectionEntry>::default().is_empty());
}
