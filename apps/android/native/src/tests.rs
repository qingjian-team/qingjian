//! 默认用独立小词库验证桥接语义，也可指定正式数据做集成回归。

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::bridge::Bridge;

fn with_bridge(label: &str, test: impl FnOnce(Bridge, &Path, &Path)) {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "qingjian-android-{label}-{}-{stamp}",
        std::process::id()
    ));
    let user = root.join("user");
    let data = std::env::var_os("QINGJIAN_ANDROID_TEST_DATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("data"));
    if std::env::var_os("QINGJIAN_ANDROID_TEST_DATA").is_none() {
        std::fs::create_dir_all(&data).unwrap();
        // 加载器按内容识别 TSV / QJ；夹具不需要下载产品数据。
        std::fs::write(
            data.join("dict.qj"),
            "开发\tkai fa\t9000\n开\tkai\t8000\n你\tni\t9000\n好\thao\t9000\n你好\tni hao\t10000\n",
        )
        .unwrap();
        std::fs::write(
            data.join("glossary-en.qj"),
            "开发\tv. develop\n你\tpron. you\n好\tadj. good\n你好\tint. hello\n",
        )
        .unwrap();
    }
    let bridge = Bridge::open(&data, &user).expect("load test data");
    test(bridge, &data, &user);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn dictionary_and_english_annotations() {
    with_bridge("gloss", |mut bridge, _, _| {
        let result = bridge.query("kaifa");
        let candidates = result["candidates"].as_array().unwrap();
        let word = candidates
            .iter()
            .find(|c| c["text"] == "开发")
            .expect("开发");
        assert!(candidates.iter().all(|c| c["gloss"] == ""));
        let annotated = bridge.annotate("kaifa");
        let translated = annotated["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["text"] == "开发")
            .unwrap();
        assert!(!translated["gloss"].as_str().unwrap().is_empty());
        assert_eq!(translated["consumed"], word["consumed"]);
        assert_eq!(word["consumed"], 5);
    });
}

#[test]
fn partial_candidate_keeps_remaining_pinyin() {
    with_bridge("partial", |mut bridge, _, _| {
        let result = bridge.query("nihao");
        let candidates = result["candidates"].as_array().unwrap();
        let word = candidates.iter().find(|c| c["text"] == "你").expect("你");
        assert_eq!(word["consumed"], 2);
        assert!(
            candidates
                .iter()
                .any(|c| c["text"] == "你好" && c["consumed"] == 5)
        );
    });
}

#[test]
fn confirmed_selection_is_persisted_and_reloaded() {
    with_bridge("learn", |mut engine, data, user| {
        engine.learn("开发", "kaifa");
        drop(engine);
        assert!(user.join("frequency.tsv").is_file());
        let mut reloaded = Bridge::open(data, user).unwrap();
        assert!(
            reloaded.query("kaifa")["candidates"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["text"] == "开发")
        );
    });
}

#[test]
fn invalid_input_and_overlong_composition_are_safe() {
    with_bridge("invalid", |mut bridge, _, _| {
        assert_eq!(
            bridge.query("你好")["candidates"].as_array().unwrap().len(),
            0
        );
        assert_eq!(
            bridge.query(&"a".repeat(97))["candidates"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        assert!(
            bridge.query("")["candidates"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    });
}

#[test]
fn stale_annotations_do_not_replace_current_input() {
    with_bridge("stale", |mut bridge, _, _| {
        bridge.query("kaifa");
        bridge.query("nihao");
        assert!(
            bridge.annotate("kaifa")["candidates"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(
            !bridge.annotate("nihao")["candidates"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    });
}
