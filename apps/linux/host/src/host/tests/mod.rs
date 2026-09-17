//! Host 行为测试：走 assets/sample 真数据；共用助手在本文件，按主题分子模块。

mod config;
mod data;
mod keys;
mod model;
mod release;

use super::*;

fn sample_host() -> Host {
    host_with(qingjian_platform::Config::default())
}

/// 每个 Host 一个独立临时数据目录：init 之后引擎会往数据目录写统计/日志，
/// 绝不能拿 assets/sample 当数据目录（会把落盘文件写进仓库）。
fn host_with(config: qingjian_platform::Config) -> Host {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static SEQ: AtomicUsize = AtomicUsize::new(0);
    let dir = temp_data_dir(&format!("h{}", SEQ.fetch_add(1, Ordering::Relaxed)));
    Host::init(dir, Some(config)).expect("样例数据应能装配")
}

fn type_str(h: &mut Host, s: &str) {
    for c in s.chars() {
        assert!(h.key(c as u32, 0, false), "字母键应被吞掉:{c}");
    }
}

/// 建一个独立的临时数据目录（拷样例数据），测试写盘类功能不弄脏 assets/sample。
fn temp_data_dir(tag: &str) -> PathBuf {
    let sample = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../assets/sample");
    let dir = std::env::temp_dir().join(format!("qj-test-{}-{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["dict.tsv", "english.tsv", "glossary-en.tsv"] {
        std::fs::copy(sample.join(f), dir.join(f)).unwrap();
    }
    dir
}
