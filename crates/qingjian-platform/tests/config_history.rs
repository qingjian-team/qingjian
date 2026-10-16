//! 历代配置回归：每个发版（与出过问题的开发版）第一次运行时写给用户的配置模板，现在的版本都必须读得进。
//! 样本在 `tests/config_history/`，由同目录的 `extract.py` 从 git 历史还原，发版后补一份新 tag 的；读不进时整份配置会回缺省（#625 → #661）。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use qingjian_core::TraditionalVariant;
use qingjian_platform::Config;

fn samples() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/config_history");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("样本目录")
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    files.sort();
    files
}

fn load(text: &str, name: &str) -> Config {
    // 测试并行跑，同一份样本会被两个测试同时读：目录名带序号，免得一边删一边写
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let serial = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "qingjian-config-history-{}-{serial}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    std::fs::write(&path, text).unwrap();
    let config = Config::load(&path).unwrap_or_else(|error| panic!("{name} 读不进：{error}"));
    std::fs::remove_dir_all(&dir).unwrap();
    config
}

/// 样本被误删时别悄悄地全过：至少是 0.1.0 到 0.2.1 各平台加开发版的数量。
#[test]
fn samples_are_present() {
    let count = samples().len();
    assert!(
        count >= 20,
        "只有 {count} 份历代配置样本，tests/config_history/ 是不是被删了"
    );
}

/// 原样读：每一份都读得进，繁体输出是缺省的简体。
#[test]
fn every_released_template_loads() {
    for path in samples() {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let config = load(&std::fs::read_to_string(&path).unwrap(), &name);
        assert_eq!(
            config.general.traditional(),
            TraditionalVariant::Off,
            "{name}"
        );
    }
}

/// 改过繁体那一项的：当时的设置程序写下的值，现在要读成同一档（0.2.1 的布尔、#625 开发版的字符串）。
#[test]
fn traditional_edits_from_old_settings_still_apply() {
    let edits = [
        (
            "traditional = false",
            "traditional = true",
            TraditionalVariant::Taiwan,
        ),
        (
            "traditional = \"off\"",
            "traditional = \"hongkong\"",
            TraditionalVariant::Hongkong,
        ),
        (
            "traditional_variant = \"off\"",
            "traditional_variant = \"standard\"",
            TraditionalVariant::Standard,
        ),
    ];
    let mut checked = 0;
    for path in samples() {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let text = std::fs::read_to_string(&path).unwrap();
        for (from, to, expected) in edits {
            if text.lines().any(|line| line == from) {
                let config = load(&text.replace(from, to), &name);
                assert_eq!(config.general.traditional(), expected, "{name}：{to}");
                checked += 1;
            }
        }
    }
    // 0.2.1 的三个平台写的是布尔，开发版写的是字符串，两种都得有样本覆盖到
    assert!(checked >= 6, "只核对了 {checked} 份改过繁体的配置");
}
