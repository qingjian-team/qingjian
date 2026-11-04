use super::*;

#[test]
fn horizontal_grid_is_off_unless_switched_on() {
    assert!(!GeneralConfig::default().horizontal_grid);
    let general: GeneralConfig = toml::from_str("layout = \"horizontal\"\n").unwrap();
    assert!(!general.horizontal_grid);
    let general: GeneralConfig = toml::from_str("horizontal_grid = true\n").unwrap();
    assert!(general.horizontal_grid);
    assert!(!GeneralConfig::default().grid_page_keys);
    let general: GeneralConfig = toml::from_str("grid_page_keys = true\n").unwrap();
    assert!(general.grid_page_keys);
}

#[test]
fn english_candidate_limits_are_sanitized() {
    let mut general = GeneralConfig::default();
    assert_eq!(
        general.english_max_in_chinese(),
        qingjian_core::ENGLISH_CANDIDATES_MAX
    );
    assert_eq!(general.english_min_position(), 1);
    general.english_max_in_chinese = 0;
    general.english_min_position = 0;
    assert_eq!(general.english_max_in_chinese(), 1);
    assert_eq!(general.english_min_position(), 1);
    general.english_max_in_chinese = 99;
    general.english_min_position = 99;
    assert_eq!(
        general.english_max_in_chinese(),
        qingjian_core::ENGLISH_CANDIDATES_MAX
    );
    assert_eq!(general.english_min_position(), MAX_PAGE_SIZE);
}

#[test]
fn page_size_and_keys_are_sanitized() {
    let mut general = GeneralConfig::default();
    assert_eq!(general.page_size(), 9);
    assert_eq!(general.page_keys(), ('[', ']'));
    general.page_size = 0;
    general.page_keys = ",.".to_owned();
    assert_eq!(general.page_size(), 1);
    assert_eq!(general.page_keys(), (',', '.'));
    general.page_size = 42;
    general.page_keys = "ab".to_owned();
    assert_eq!(general.page_size(), 9);
    assert_eq!(general.page_keys(), ('[', ']'));
    general.page_keys = ",,".to_owned();
    assert_eq!(general.page_keys(), ('[', ']'));
}

/// 组句中标点先上屏候选缺省关（`[general] punct_commits`），保持标点进英文直输段的老行为。
#[test]
fn punct_commits_defaults_off() {
    assert!(!GeneralConfig::default().punct_commits);
    let general: GeneralConfig = toml::from_str("punct_commits = true\n").unwrap();
    assert!(general.punct_commits);
}

#[test]
fn double_shift_third_input_defaults_off() {
    assert_eq!(
        GeneralConfig::default().double_shift_input,
        ThirdInputMethod::Off
    );
    let general: GeneralConfig = toml::from_str("double_shift_input = \"vietnamese\"\n").unwrap();
    assert_eq!(general.double_shift_input, ThirdInputMethod::Vietnamese);
}

#[test]
fn aux_code_key_falls_back_to_the_default() {
    let mut general = GeneralConfig::default();
    assert_eq!(general.aux_code_key(), ';');
    assert!(!general.aux_code_show);
    general.aux_code_key = "/".to_owned();
    assert_eq!(general.aux_code_key(), '/');
    for bad in ["", "ab", "a", "1", "[", "中"] {
        general.aux_code_key = bad.to_owned();
        assert_eq!(general.aux_code_key(), ';', "{bad}");
    }
}

/// 码删空保持辅码态的开关缺省开（`[general] aux_code_keep_empty`）。
#[test]
fn aux_code_keep_empty_defaults_on() {
    assert!(GeneralConfig::default().aux_code_keep_empty);
}

/// 词性标注缺省开（旧配置文件没这一项时行为不变），显式写 false 才关。
#[test]
fn translation_pos_defaults_on() {
    assert!(GeneralConfig::default().translation_pos);
    let general: GeneralConfig = toml::from_str("").unwrap();
    assert!(general.translation_pos);
    let general: GeneralConfig = toml::from_str("translation_pos = false\n").unwrap();
    assert!(!general.translation_pos);
}

#[test]
fn scheme_defaults_to_pinyin_and_unknown_names_fall_back() {
    let mut general = GeneralConfig::default();
    assert_eq!(general.scheme(), Scheme::Pinyin);
    general.scheme = "xiaohe".to_owned();
    assert_eq!(general.shuangpin(), Some(ShuangpinScheme::Xiaohe));
    general.scheme = " Sogou ".to_owned();
    assert_eq!(general.shuangpin(), Some(ShuangpinScheme::Sogou));
    general.scheme = "xiaolang".to_owned();
    assert_eq!(general.shuangpin(), Some(ShuangpinScheme::Xiaolang));
    general.scheme = "none".to_owned();
    assert_eq!(general.scheme(), Scheme::Off);
    assert!(!general.scheme().is_on());
    assert_eq!(general.custom_shuangpin(), None);
    general.scheme = " my-sp ".to_owned();
    assert_eq!(general.scheme(), Scheme::Custom);
    assert_eq!(general.custom_shuangpin().as_deref(), Some("my-sp"));
    assert_eq!(general.shuangpin(), None);
    general.scheme = "Flypy!".to_owned();
    assert_eq!(general.scheme(), Scheme::Pinyin);
    assert_eq!(general.custom_shuangpin(), None);
}

#[test]
fn the_two_axes_are_independent_and_their_combination_is_mixed_input() {
    let mut general = GeneralConfig::default();
    // 缺省：全拼，不开形码
    assert_eq!(general.scheme(), Scheme::Pinyin);
    assert_eq!(general.code(), None);
    assert!(!general.mixed());

    // 双拼 alone
    general.scheme = "xiaohe".to_owned();
    assert!(!general.mixed());
    // 形码 alone：拼音侧关掉
    general.scheme = "none".to_owned();
    general.code = "huma".to_owned();
    assert_eq!(general.code().as_deref(), Some("huma"));
    assert!(!general.mixed());
    // 组合：两边都开 = 混输
    general.scheme = "xiaohe".to_owned();
    assert!(general.mixed());
    // 关形码不影响拼音侧
    general.code = "off".to_owned();
    assert_eq!(general.code(), None);
    assert_eq!(general.scheme(), Scheme::Shuangpin(ShuangpinScheme::Xiaohe));
}

#[test]
fn files_written_before_the_code_key_keep_their_wubi() {
    let parse = |text: &str| toml::from_str::<GeneralConfig>(text).unwrap().code();
    assert_eq!(parse("wubi = \"wubi86\"\n").as_deref(), Some("wubi86"));
    assert_eq!(parse("wubi = \"\"\n"), None);
    assert_eq!(parse(""), None);
    // 新键写了就以它为准，`off` 关掉也不回头看旧键
    assert_eq!(parse("code = \"off\"\nwubi = \"wubi86\"\n"), None);
    assert_eq!(
        parse("code = \"huma\"\nwubi = \"wubi86\"\n").as_deref(),
        Some("huma")
    );
}

#[test]
fn the_old_wubi_scheme_spelling_still_turns_wubi_on() {
    // 2026-09-16 之前五笔是 `scheme = "wubi86"`（单选）。升级后两个轴都得跟上，
    // 否则老配置会变成「拼音关 + 形码关」，一个字都打不出来。
    let general = GeneralConfig {
        scheme: "wubi86".to_owned(),
        ..GeneralConfig::default()
    };
    assert_eq!(general.scheme(), Scheme::Off);
    assert_eq!(general.code().as_deref(), Some("wubi86"));
    assert!(!general.mixed());
}

#[test]
fn files_written_before_the_scheme_key_keep_their_scheme() {
    let parse = |text: &str| toml::from_str::<GeneralConfig>(text).unwrap().scheme();
    assert_eq!(
        parse("shuangpin = \"xiaohe\"\n"),
        Scheme::Shuangpin(ShuangpinScheme::Xiaohe)
    );
    assert_eq!(parse("zhuyin = true\n"), Scheme::Zhuyin);
    assert_eq!(parse("shuangpin = \"\"\nzhuyin = false\n"), Scheme::Pinyin);
    assert_eq!(parse(""), Scheme::Pinyin);
    // 新键写了就以它为准
    assert_eq!(
        parse("scheme = \"pinyin\"\nshuangpin = \"xiaohe\"\n"),
        Scheme::Pinyin
    );
}

/// 0.2.1 及以前的 `traditional = true` 照旧出台湾字形；开发版在旧键里写的字符串也认；新键优先；不认识的写法只让这一项回简体。
#[test]
fn traditional_variant_wins_over_the_legacy_key() {
    let parse = |text: &str| toml::from_str::<GeneralConfig>(text).unwrap().traditional();
    assert_eq!(parse("traditional = true"), TraditionalVariant::Taiwan);
    assert_eq!(parse("traditional = false"), TraditionalVariant::Off);
    assert_eq!(parse(""), TraditionalVariant::Off);
    assert_eq!(
        parse("traditional = true\ntraditional_variant = \"off\""),
        TraditionalVariant::Off
    );
    assert_eq!(
        parse("traditional_variant = \"hongkong\""),
        TraditionalVariant::Hongkong
    );
    assert_eq!(
        parse("traditional_variant = \"macau\"\npage_size = 5"),
        TraditionalVariant::Off
    );
    assert_eq!(parse("traditional = \"off\""), TraditionalVariant::Off);
    let last = |text: &str| {
        toml::from_str::<GeneralConfig>(text)
            .unwrap()
            .last_traditional()
    };
    assert_eq!(last(""), TraditionalVariant::Taiwan);
    assert_eq!(
        last("last_traditional = \"off\""),
        TraditionalVariant::Taiwan
    );
    assert_eq!(
        last("last_traditional = \"hongkong\""),
        TraditionalVariant::Hongkong
    );
    assert_eq!(
        parse("traditional = \"hongkong\""),
        TraditionalVariant::Hongkong
    );
    assert_eq!(
        parse("traditional = \"hongkong\"\ntraditional_variant = \"standard\""),
        TraditionalVariant::Standard
    );
    let unknown = toml::from_str::<GeneralConfig>("traditional = \"macau\"\npage_size = 5")
        .expect("旧键写错不能让整个配置解析失败");
    assert_eq!(unknown.traditional(), TraditionalVariant::Off);
    assert_eq!(unknown.page_size, 5);
}

#[test]
fn legacy_shuangpin_and_zhuyin_keys_are_read_but_scheme_wins() {
    let mut general = GeneralConfig {
        scheme: String::new(),
        ..GeneralConfig::default()
    };
    assert_eq!(general.scheme(), Scheme::Pinyin);
    general.shuangpin = Some("xiaohe".to_owned());
    assert_eq!(general.scheme(), Scheme::Shuangpin(ShuangpinScheme::Xiaohe));
    general.zhuyin = Some(true);
    assert_eq!(general.scheme(), Scheme::Zhuyin);
    assert!(general.is_zhuyin());
    // 旧配置里两个都写是不合法的，新键写了就不看它们
    general.scheme = "pinyin".to_owned();
    assert_eq!(general.scheme(), Scheme::Pinyin);
    assert!(!general.is_zhuyin());
}

#[test]
fn ui_language_defaults_to_simplified_chinese_and_system_follows_the_system() {
    assert_eq!(GeneralConfig::default().ui_locale(), "zh-Hans");
    let general: GeneralConfig = toml::from_str("ui_language = \"system\"\n").unwrap();
    assert_eq!(general.ui_locale(), "");
    let general: GeneralConfig = toml::from_str("ui_language = \"vi\"\n").unwrap();
    assert_eq!(general.ui_locale(), "vi");
}
