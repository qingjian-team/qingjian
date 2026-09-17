//! 配置测试：热加载、按应用设置、全半角与候选开关。

use super::*;

#[test]
fn config_hot_reload_applies_fuzzy() {
    let dir = std::env::temp_dir().join(format!("qj-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = dir.join("config.toml");
    std::fs::write(&cfg, "[general]\n").unwrap();
    let mut h = sample_host();
    h.watch_config(cfg.clone());
    // 未开模糊：si 出不了「是」（sh 声母）
    type_str(&mut h, "si");
    let has_shi = |h: &Host| {
        (0..h.layout.len())
            .filter_map(|i| h.layout.candidate(i))
            .any(|c| c.text == "是")
    };
    assert!(!has_shi(&h), "模糊未开时 si 不应出「是」");
    h.key(0xff1b, 0, false); // Esc 清空
    // 改配置开 s_sh，回拨探测节流与 mtime 后重新输入
    std::fs::write(&cfg, "[fuzzy]\ns_sh = true\n").unwrap();
    h.config_mtime = None; // 模拟 mtime 变化（文件系统秒级精度不可靠）
    h.last_config_check = std::time::Instant::now() - CONFIG_CHECK_INTERVAL;
    type_str(&mut h, "si");
    assert!(has_shi(&h), "热加载 s_sh 后 si 应出「是」");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn full_width_punctuation_off_passes_half_width() {
    // 配置关掉全角标点：空闲时敲逗号不再转全角，原样透传。
    let mut config = qingjian_platform::Config::default();
    config.general.full_width_punctuation = false;
    let mut h = host_with(config);
    assert!(!h.key(0x2c, 0, false), "全角标点关掉后逗号应透传");
    assert!(h.pending_commit.is_none());
}

#[test]
fn custom_phrases_config_applies() {
    // 自定义短语：敲输入码，固定位置出短语。
    let config = qingjian_platform::Config {
        custom_phrases: vec![qingjian_core::CustomPhrase {
            code: "addr".to_owned(),
            text: "青简大道 1 号".to_owned(),
            position: 1,
            enabled: true,
        }],
        ..Default::default()
    };
    let mut h = host_with(config);
    type_str(&mut h, "addr");
    let first = h.layout.candidate(0).map(|c| c.text.clone());
    assert_eq!(
        first.as_deref(),
        Some("青简大道 1 号"),
        "自定义短语应出现在第 1 格"
    );
}

#[test]
fn shuangpin_config_applies() {
    // 双拼（小鹤）：u=sh、i=i，敲 ui 应出「是」；全拼下 ui 不该出。
    let mut config = qingjian_platform::Config::default();
    config.general.shuangpin = "xiaohe".to_owned();
    let mut h = host_with(config);
    type_str(&mut h, "ui");
    let has_shi = (0..h.layout.len())
        .filter_map(|i| h.layout.candidate(i))
        .any(|c| c.text == "是");
    assert!(has_shi, "小鹤双拼下 ui 应出「是」");
}

#[test]
fn english_candidates_off_is_pure_passthrough() {
    // [general] english_candidates = false：英文模式纯直通，字母不进缓冲区。
    let mut config = qingjian_platform::Config::default();
    config.general.english_candidates = false;
    let mut h = host_with(config);
    h.key(0xffe1, 0, false);
    h.key(0xffe1, 1, true);
    assert!(h.engine.english_mode(), "轻点仍切到英文模式");
    assert!(!h.key(0x6b, 0, false), "英文候选关着:字母应透传");
    assert!(!h.composing(), "纯直通不组句");
    assert!(h.pending_commit.is_none());
}

#[test]
fn config_hot_reload_applies_new_fields() {
    // 热加载也要覆盖新接的字段：翻页键与全角标点开关。
    let dir = std::env::temp_dir().join(format!("qj-test-fields-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = dir.join("config.toml");
    std::fs::write(&cfg, "[general]\n").unwrap();
    let mut h = sample_host();
    h.watch_config(cfg.clone());
    std::fs::write(
        &cfg,
        "[general]\npage_keys = \",.\"\nfull_width_punctuation = false\n",
    )
    .unwrap();
    h.config_mtime = None;
    h.last_config_check = std::time::Instant::now() - CONFIG_CHECK_INTERVAL;
    type_str(&mut h, "s");
    assert!(h.page_count() > 1);
    assert!(h.key(0x2e, 0, false), "热加载后 . 应翻页");
    assert_eq!(h.page, 1);
    h.key(0xff1b, 0, false);
    assert!(!h.key(0x2c, 0, false), "热加载后空闲逗号应透传(全角已关)");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn per_app_english_candidates_off() {
    // [apps] english_candidates_off 列出的应用里英文模式纯直通；别的应用不受影响。
    let config = qingjian_platform::Config {
        apps: qingjian_platform::AppsConfig::with_english_candidates_off(&["konsole"]),
        ..Default::default()
    };
    let mut h = host_with(config);
    h.key(0xffe1, 0, false);
    h.key(0xffe1, 1, true);
    assert!(h.engine.english_mode());
    h.set_program("konsole");
    assert!(!h.key(0x6b, 0, false), "名单内应用里字母应透传");
    assert!(!h.composing());
    h.set_program("kate");
    assert!(h.key(0x6b, 0, false), "名单外应用里应正常给英文候选");
    assert!(h.composing());
}

#[test]
fn preedit_display_follows_config() {
    // [general] preedit 决定拼音行显示位置（0=两处 1=只行内 2=只窗口），shim 按它画。
    let mut h = sample_host();
    assert_eq!(h.preedit_display(), 0, "缺省两处都显示");
    let mut config = qingjian_platform::Config::default();
    config.general.preedit = qingjian_platform::PreeditMode::Window;
    h.apply_config(&config);
    assert_eq!(h.preedit_display(), 2, "配置只在窗口后应返回 2");
}
