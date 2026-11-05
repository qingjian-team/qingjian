use std::path::PathBuf;

use super::*;
use crate::Writer;

/// 测试用的临时目录，drop 时删除。
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("qingjian-qjz-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write_qj(path: &Path, kind: Kind, name: &str) -> Metadata {
    let metadata = Metadata {
        name: name.to_owned(),
        license: "CC-BY-SA-4.0".to_owned(),
        entries: 1,
        ..Metadata::default()
    };
    Writer::new(kind, &metadata)
        .unwrap()
        .section(*b"TEXT", "医学".as_bytes())
        .write_to(path)
        .unwrap();
    metadata
}

#[test]
fn round_trips_and_reports_kind_and_metadata() {
    let dir = TempDir::new("round-trip");
    let qj = dir.0.join("medicine.qj");
    let metadata = write_qj(&qj, Kind::Dictionary, "青简领域词库·医学");
    let qjz = dir.0.join("medicine.qjz");
    compress(&qj, &qjz).unwrap();
    assert!(is_qjz(&qjz));
    assert!(!is_qjz(&qj));
    assert!(std::fs::metadata(&qjz).unwrap().len() > 0);

    let target = dir.0.join("out.qj");
    let (kind, unpacked) = unpack(&qjz, &target).unwrap();
    assert_eq!(kind, Kind::Dictionary);
    assert_eq!(unpacked, metadata);
    assert_eq!(std::fs::read(&target).unwrap(), std::fs::read(&qj).unwrap());
    assert!(!target.with_extension("qj.part").exists());
}

#[test]
fn reports_glossary_kind() {
    let dir = TempDir::new("glossary");
    let qj = dir.0.join("glossary-ja.qj");
    write_qj(&qj, Kind::Glossary, "青简释义表·日语");
    let qjz = dir.0.join("glossary-ja.qjz");
    compress(&qj, &qjz).unwrap();
    let (kind, _) = unpack(&qjz, &dir.0.join("glossary-ja.out.qj")).unwrap();
    assert_eq!(kind, Kind::Glossary);
}

#[test]
fn refuses_to_compress_non_qj() {
    let dir = TempDir::new("not-qj");
    let tsv = dir.0.join("words.tsv");
    std::fs::write(&tsv, "医学\tyi xue\t100\n").unwrap();
    assert!(matches!(
        compress(&tsv, &dir.0.join("words.qjz")),
        Err(FormatError::BadMagic)
    ));
}

#[test]
fn rejects_unknown_compression_and_keeps_old_target() {
    let dir = TempDir::new("unknown");
    let source = dir.0.join("plain.qjz");
    std::fs::write(&source, b"not compressed at all").unwrap();
    let target = dir.0.join("kept.qj");
    std::fs::write(&target, b"old").unwrap();
    assert!(matches!(
        unpack(&source, &target),
        Err(FormatError::UnknownCompression)
    ));
    assert_eq!(std::fs::read(&target).unwrap(), b"old");
}

#[test]
fn rejects_compressed_garbage_and_cleans_up() {
    let dir = TempDir::new("garbage");
    let source = dir.0.join("garbage.qjz");
    let mut encoder = GzEncoder::new(File::create(&source).unwrap(), Compression::fast());
    encoder
        .write_all(b"QINGJIAN but the rest is not a valid header or section table")
        .unwrap();
    encoder.finish().unwrap();
    let target = dir.0.join("garbage.qj");
    assert!(unpack(&source, &target).is_err());
    assert!(!target.exists());
    assert!(!target.with_extension("qj.part").exists());
}
