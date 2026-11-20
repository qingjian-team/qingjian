//! Golden fixture 回归：完整旧格式配置读入逐键断言、序列化键集合相等、缺省补齐与错误路径。
//!
//! 样本在 `tests/fixtures/general_full.toml`，覆盖全部 64 个 `[general]` 键；配置拆分前后
//! 这份文件读出来的值必须逐项相等，序列化回去的键集合必须与样本完全一致。

use std::collections::BTreeSet;
use std::path::Path;

use qingjian_core::TraditionalVariant;
use qingjian_platform::{
    Appearance, CandidateRenderer, Config, EmojiCount, FontSize, KeyHintMode, LayoutMode,
    LogLevel, NumpadDigit, PreeditMode, ShiftLetter, ThirdInputMethod,
};

fn fixture() -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    std::fs::read_to_string(dir.join("general_full.toml")).unwrap()
}

/// 取出一份 TOML 文本里 `[general]` 分节的键集合。
fn general_keys(text: &str) -> BTreeSet<String> {
    let value: toml::Value = toml::from_str(text).unwrap();
    value
        .get("general")
        .and_then(toml::Value::as_table)
        .unwrap_or_else(|| panic!("样本里应有 [general] 分节"))
        .keys()
        .cloned()
        .collect()
}

/// (a) 完整旧格式配置读入：64 个键逐项断言，值与样本一致。
#[test]
fn golden_fixture_loads_with_every_key_asserted() {
    let config: Config = toml::from_str(&fixture()).unwrap();
    let general = &config.general;

    // 译文与候选内容
    assert_eq!(general.candidates.learning_language, "ja");
    assert!(!general.candidates.translation_pos);
    assert!(general.candidates.show_tones);
    assert_eq!(general.candidates.page_size, 5);
    assert_eq!(general.candidates.page_keys, ",.");
    assert_eq!(general.candidates.preedit, PreeditMode::Window);
    assert!(!general.candidates.english_candidates);
    assert!(general.candidates.chinese_first);
    assert!(!general.candidates.spelling_correction);
    assert!(!general.candidates.english_in_chinese);
    assert_eq!(general.candidates.english_max_in_chinese, 2);
    assert_eq!(general.candidates.english_min_position, 3);
    assert!(general.candidates.show_english_pronunciation);
    assert_eq!(general.candidates.emoji, EmojiCount::new(2));
    assert_eq!(general.candidates.kaomoji, EmojiCount::new(2));
    assert_eq!(general.candidates.symbols, EmojiCount::new(2));
    assert!(general.candidates.auto_space);

    // 外观与候选窗布局
    assert_eq!(general.display.appearance, Some(Appearance::Dark));
    assert_eq!(general.display.theme, "sakura");
    assert_eq!(general.display.layout, LayoutMode::Horizontal);
    assert!(general.display.horizontal_grid);
    assert!(general.display.grid_page_keys);
    assert!(general.display.vertical_reverse_above);
    assert_eq!(general.display.renderer, CandidateRenderer::System);
    assert_eq!(general.display.font, "LXGW WenKai");
    assert_eq!(general.display.candidate_font_size, FontSize(20.0));
    assert_eq!(general.display.annotation_font_size, FontSize(13.5));
    assert_eq!(general.display.candidate_font_weight, 500);
    assert!(!general.display.animations);
    assert_eq!(general.display.key_hint, KeyHintMode::Always);
    assert_eq!(general.display.key_hint_opacity, 60);
    assert_eq!(general.display.key_hint_size, 120);

    // 输入模式与标点
    assert_eq!(general.input.shift_letter, ShiftLetter::Compose);
    assert_eq!(general.input.numpad_digit, NumpadDigit::Direct);
    assert!(!general.input.english_mode);
    assert!(general.input.default_english);
    assert!(general.input.remember_mode_per_app);
    assert_eq!(general.input.double_shift_input, ThirdInputMethod::Vietnamese);
    assert!(!general.input.full_width_punctuation);
    assert!(general.input.english_full_width_punctuation);
    assert!(!general.input.pair_punctuation);
    assert!(general.input.english_pair_punctuation);
    assert_eq!(general.input.half_width_symbols, "`");
    assert!(general.input.punct_commits);
    assert!(!general.input.hold_selections);

    // 编码方案与辅码（含旧键）
    assert_eq!(general.schemes.aux_code_key, "/");
    assert!(general.schemes.aux_code_show);
    assert!(!general.schemes.aux_code_keep_empty);
    assert_eq!(general.schemes.scheme, "xiaohe");
    assert!(general.schemes.shuangpin_raw_preedit);
    assert!(general.schemes.shuangpin_raw_window);
    assert!(general.schemes.shuangpin_full_pinyin);
    assert_eq!(general.schemes.code, "wubi86");
    assert_eq!(general.schemes.wubi, Some("wubi86".to_owned()));
    assert_eq!(general.schemes.shuangpin, Some("xiaohe".to_owned()));
    assert_eq!(general.schemes.zhuyin, Some(true));

    // 繁体、界面语言与学习调试
    assert_eq!(general.advanced.traditional_variant, "hongkong");
    assert_eq!(general.traditional(), TraditionalVariant::Hongkong);
    assert_eq!(general.advanced.last_traditional, "taiwan");
    assert_eq!(general.advanced.ui_language, "vi");
    assert_eq!(general.advanced.log_level, LogLevel::Debug);
    assert!(!general.advanced.input_log);
    assert!(!general.advanced.learning);
    assert!(!general.advanced.system_text_replacements);
}

/// (b) 读入后再序列化：`[general]` 键集合与样本完全相等，不多键、少键，层级不变深。
#[test]
fn reserialized_fixture_keeps_the_same_general_key_set() {
    let source = fixture();
    let config: Config = toml::from_str(&source).unwrap();
    let serialized = toml::to_string(&config).unwrap();

    let source_keys = general_keys(&source);
    let serialized_keys = general_keys(&serialized);
    assert_eq!(source_keys.len(), 64, "样本应覆盖全部 64 个键");
    assert_eq!(
        source_keys, serialized_keys,
        "序列化前后 [general] 键集合不一致"
    );

    // 再读一遍必须与第一次相等（值也 roundtrip）。
    let restored: Config = toml::from_str(&serialized).unwrap();
    assert_eq!(config, restored);
}

/// (c) 只写一部分键：其余键按缺省补齐，不串到别的分节。
#[test]
fn partial_section_fills_the_rest_with_defaults() {
    // 分节存在但缺键时走字段级 serde 默认（与整个分节缺失时的 `Default` impl 略有不同，见 general 单测）。
    let baseline: Config = toml::from_str("[general]\n").unwrap();
    let config: Config = toml::from_str("[general]\npage_size = 7\n").unwrap();
    assert_eq!(config.general.candidates.page_size, 7);
    let mut expected = baseline.general.clone();
    expected.page_size = 7;
    assert_eq!(config.general, expected);
}

/// (d) 错误路径：未知键忽略；类型错误报错；越界值由取值方法钳制，不让整份配置失败。
#[test]
fn unknown_keys_wrong_types_and_out_of_range_keep_old_behavior() {
    let baseline: Config = toml::from_str("[general]\n").unwrap();

    // 未知键照旧忽略。
    let ignored: Config = toml::from_str("[general]\nno_such_key = 1\n").unwrap();
    assert_eq!(ignored, baseline);

    // 类型写错照旧报错，不静默吞掉笔误。
    assert!(toml::from_str::<Config>("[general]\npage_size = \"x\"").is_err());
    assert!(toml::from_str::<Config>("[general]\nlayout = 123").is_err());

    // 越界数值解析不报错，取值时夹回合法范围。
    let clamped: Config = toml::from_str("[general]\npage_size = 42\n").unwrap();
    assert_eq!(clamped.general.page_size(), 9);
}
