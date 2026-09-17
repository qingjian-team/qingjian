//! 数据装配测试：分层查找、用户词库与输入日志。

use super::*;

#[test]
fn user_extra_dictionary_appears_in_candidates() {
    // 用户词库：user-dicts/ 下的 TSV 词条应进候选。
    let dir = temp_data_dir("extradict");
    std::fs::create_dir_all(dir.join("user-dicts")).unwrap();
    std::fs::write(
        dir.join("user-dicts/mine.tsv"),
        "青简验证\tqing jian yan zheng\t500\n",
    )
    .unwrap();
    let mut h = Host::init(dir.clone(), Some(qingjian_platform::Config::default()))
        .expect("样例数据应能装配");
    type_str(&mut h, "qingjianyanzheng");
    let has = (0..h.layout.len())
        .filter_map(|i| h.layout.candidate(i))
        .any(|c| c.text == "青简验证");
    assert!(has, "用户词库的词应出现在候选里");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn input_log_writes_when_enabled() {
    // 输入日志（缺省开）：上屏后 input-log.jsonl 应落盘；配置关掉则不再新增。
    let dir = temp_data_dir("inputlog");
    let mut h = Host::init(dir.clone(), Some(qingjian_platform::Config::default()))
        .expect("样例数据应能装配");
    type_str(&mut h, "ni");
    h.key(0x20, 0, false);
    h.pending_commit.take();
    h.reset(); // 落盘时机与切窗对齐
    let log = dir.join("input-log.jsonl");
    assert!(log.exists(), "输入日志开着时应写 input-log.jsonl");
    assert!(
        std::fs::metadata(&log).unwrap().len() > 0,
        "日志文件应有内容"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn data_lookup_prefers_user_layer_over_dist() {
    // 分层查找：随包数据在 dist/，用户层（根）同名文件盖过它；
    // 层内 .qj 优先 .tsv，但用户层整层先于随包层——用户放个 tsv 也能盖过随包 qj。
    let dir = temp_data_dir("layers");
    std::fs::create_dir_all(dir.join("dist")).unwrap();
    std::fs::write(dir.join("dist/probe.qj"), "").unwrap();
    std::fs::write(dir.join("dist/probe.tsv"), "").unwrap();
    assert_eq!(
        find_data(&dir, "probe").unwrap(),
        dir.join("dist/probe.qj"),
        "只有随包层时取 dist,且 .qj 优先"
    );
    std::fs::write(dir.join("probe.tsv"), "").unwrap();
    assert_eq!(
        find_data(&dir, "probe").unwrap(),
        dir.join("probe.tsv"),
        "用户层整层先于随包层"
    );
    std::fs::write(dir.join("dist/emoji-probe.tsv"), "").unwrap();
    assert_eq!(
        find_file(&dir, "emoji-probe.tsv").unwrap(),
        dir.join("dist/emoji-probe.tsv")
    );
    std::fs::write(dir.join("emoji-probe.tsv"), "").unwrap();
    assert_eq!(
        find_file(&dir, "emoji-probe.tsv").unwrap(),
        dir.join("emoji-probe.tsv"),
        "find_file 同样用户层优先"
    );
}

#[test]
fn dist_layer_alone_assembles_host() {
    // 新装机形态：随包数据全在 dist/，用户层只有学习数据——必须装得起来。
    let dir = {
        let dir = std::env::temp_dir().join(format!("qj-test-distonly-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("dist")).unwrap();
        let sample = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../assets/sample");
        for f in ["dict.tsv", "english.tsv", "glossary-en.tsv"] {
            std::fs::copy(sample.join(f), dir.join("dist").join(f)).unwrap();
        }
        dir
    };
    let mut h = Host::init(dir, Some(qingjian_platform::Config::default()))
        .expect("随包层单独在场应能装配");
    type_str(&mut h, "ni");
    assert!(!h.layout.is_empty(), "dist 层词库应出候选");
}
