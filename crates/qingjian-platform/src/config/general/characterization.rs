//! 拆分等价性 characterization：缺省值逐键断言、错误路径、边界钳制、roundtrip 与跨线程传递。
//!
//! 这些测试锁住拆分前的既有行为；改结构后断言值不允许变绿以外的调整。

use super::*;
use super::super::{
    CandidateRenderer, EmojiCount, FontSize, KeyHintMode, LayoutMode, LogLevel, NumpadDigit,
    PreeditMode, ShiftLetter, ThirdInputMethod,
};
// ── 拆分等价性 characterization：64 个 `[general]` 键的缺省值逐键断言，不抽样。 ──

/// 空分节读进来的配置：每个键都应落到反序列化缺省值。
///
/// 注意两处与 [`GeneralConfig::default`] 不同的历史行为：`appearance` 与
/// `traditional_variant` 带字段级 `#[serde(default)]`，缺键时分别是 `None` / 空串，
/// 而不是 `Default` impl 里的 `Some(System)` / `"off"`。拆分时这两条容错语义必须原样保留。
#[test]
fn every_general_key_has_its_document_default_when_omitted() {
    let parsed: GeneralConfig = toml::from_str("").unwrap();
    let empty: GeneralConfig = toml::from_str("").unwrap();

    // 译文与候选内容
    assert_eq!(parsed.candidates.learning_language, "en");
    assert_eq!(parsed.candidates.translation_pos, true);
    assert_eq!(parsed.candidates.show_tones, false);
    assert_eq!(parsed.candidates.page_size, MAX_PAGE_SIZE);
    assert_eq!(parsed.candidates.page_keys, PAGE_KEY_OPTIONS[0]);
    assert_eq!(parsed.candidates.preedit, PreeditMode::Both);
    assert_eq!(parsed.candidates.english_candidates, true);
    assert_eq!(parsed.candidates.chinese_first, false);
    assert_eq!(parsed.candidates.spelling_correction, true);
    assert_eq!(parsed.candidates.english_in_chinese, true);
    assert_eq!(
        parsed.candidates.english_max_in_chinese,
        qingjian_core::ENGLISH_CANDIDATES_MAX
    );
    assert_eq!(parsed.candidates.english_min_position, 1);
    assert_eq!(parsed.candidates.show_english_pronunciation, false);
    assert_eq!(parsed.candidates.emoji, EmojiCount::default());
    assert_eq!(parsed.candidates.kaomoji, EmojiCount::new(1));
    assert_eq!(parsed.candidates.symbols, EmojiCount::new(1));
    assert_eq!(parsed.candidates.auto_space, false);

    // 外观与候选窗布局
    assert_eq!(parsed.display.appearance, None);
    assert_eq!(parsed.display.theme, DEFAULT_THEME);
    assert_eq!(parsed.display.layout, LayoutMode::Vertical);
    assert_eq!(parsed.display.horizontal_grid, false);
    assert_eq!(parsed.display.grid_page_keys, false);
    assert_eq!(parsed.display.vertical_reverse_above, false);
    assert_eq!(parsed.display.renderer, CandidateRenderer::Qingjian);
    assert_eq!(parsed.display.font, "");
    assert_eq!(parsed.display.candidate_font_size, FontSize::default());
    assert_eq!(parsed.display.annotation_font_size, FontSize::default());
    assert_eq!(parsed.display.candidate_font_weight, 0);
    assert_eq!(parsed.display.animations, true);
    assert_eq!(parsed.display.key_hint, KeyHintMode::Off);
    assert_eq!(parsed.display.key_hint_opacity, DEFAULT_KEY_HINT_OPACITY);
    assert_eq!(parsed.display.key_hint_size, 100);

    // 输入模式与标点
    assert_eq!(parsed.input.shift_letter, ShiftLetter::Passthrough);
    assert_eq!(parsed.input.numpad_digit, NumpadDigit::Select);
    assert_eq!(parsed.input.english_mode, true);
    assert_eq!(parsed.input.default_english, false);
    assert_eq!(parsed.input.remember_mode_per_app, false);
    assert_eq!(parsed.input.double_shift_input, ThirdInputMethod::Off);
    assert_eq!(parsed.input.full_width_punctuation, true);
    assert_eq!(parsed.input.english_full_width_punctuation, false);
    assert_eq!(parsed.input.pair_punctuation, true);
    assert_eq!(parsed.input.english_pair_punctuation, false);
    assert_eq!(parsed.input.half_width_symbols, "");
    assert_eq!(parsed.input.punct_commits, false);
    assert_eq!(parsed.input.hold_selections, true);

    // 编码方案与辅码（含不再写出去的旧键）
    assert_eq!(
        parsed.schemes.aux_code_key,
        qingjian_core::DEFAULT_AUX_CODE_KEY.to_string()
    );
    assert_eq!(parsed.schemes.aux_code_show, false);
    assert_eq!(parsed.schemes.aux_code_keep_empty, true);
    assert_eq!(parsed.schemes.scheme, "");
    assert_eq!(parsed.schemes.shuangpin_raw_preedit, false);
    assert_eq!(parsed.schemes.shuangpin_raw_window, false);
    assert_eq!(parsed.schemes.shuangpin_full_pinyin, false);
    assert_eq!(parsed.schemes.code, "");
    assert_eq!(parsed.schemes.wubi, None);
    assert_eq!(parsed.schemes.shuangpin, None);
    assert_eq!(parsed.schemes.zhuyin, None);

    // 繁体、界面语言与学习调试
    assert_eq!(parsed.advanced.traditional_variant, "");
    assert_eq!(parsed.advanced.traditional, None);
    assert_eq!(parsed.advanced.last_traditional, "");
    assert_eq!(parsed.advanced.ui_language, "");
    assert_eq!(parsed.advanced.log_level, LogLevel::Info);
    assert_eq!(parsed.advanced.input_log, true);
    assert_eq!(parsed.advanced.learning, true);
    assert_eq!(parsed.advanced.system_text_replacements, true);

    // 兜底：整体与空分节解析结果相等，漏网的键在这里被抓住。
    assert_eq!(parsed, empty);

    // `Default` impl 的两条非平凡缺省单独锁住：运行时由 `Config::load` 的迁移路径补齐。
    assert_eq!(GeneralConfig::default().display.appearance, Some(Appearance::System));
    assert_eq!(
        GeneralConfig::default().advanced.traditional_variant,
        TraditionalVariant::Off.key()
    );
}

/// 未知键照旧忽略，不让整份配置解析失败。
#[test]
fn unknown_general_key_is_ignored_rather_than_rejected() {
    let parsed: GeneralConfig = toml::from_str("no_such_key = 42\n").unwrap();
    let empty: GeneralConfig = toml::from_str("").unwrap();
    assert_eq!(parsed, empty);
}

/// 类型写错的键照旧报错（不静默吞掉笔误）。
#[test]
fn wrong_type_general_key_is_an_error() {
    assert!(toml::from_str::<GeneralConfig>("page_size = \"abc\"").is_err());
    assert!(toml::from_str::<GeneralConfig>("layout = 123").is_err());
    assert!(toml::from_str::<GeneralConfig>("preedit = true").is_err());
}

/// 越界数值由取值方法夹回合法范围，原始字段保留写进来的值。
#[test]
fn out_of_range_values_are_clamped_by_accessors() {
    let general: GeneralConfig = toml::from_str(
        "page_size = 42\nenglish_max_in_chinese = 99\nenglish_min_position = 0\n",
    )
    .unwrap();
    assert_eq!(general.page_size(), MAX_PAGE_SIZE);
    assert_eq!(
        general.english_max_in_chinese(),
        qingjian_core::ENGLISH_CANDIDATES_MAX
    );
    assert_eq!(general.english_min_position(), 1);
}

/// 配置类型跨线程传递：Server 进程与壳之间按值搬运，必须 `Send + Sync`。
#[test]
fn config_and_general_are_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<GeneralConfig>();
    assert_send_sync::<crate::Config>();
}

/// 固定种子的生成式用例：随机翻布尔位、挑枚举与整数，解析 → 序列化 → 再解析必须相等。
#[test]
fn seeded_overrides_roundtrip_through_toml() {
    const BOOL_KEYS: [&str; 28] = [
        "translation_pos",
        "show_tones",
        "horizontal_grid",
        "grid_page_keys",
        "vertical_reverse_above",
        "animations",
        "english_candidates",
        "chinese_first",
        "spelling_correction",
        "english_in_chinese",
        "show_english_pronunciation",
        "auto_space",
        "english_mode",
        "default_english",
        "remember_mode_per_app",
        "english_full_width_punctuation",
        "pair_punctuation",
        "english_pair_punctuation",
        "punct_commits",
        "aux_code_show",
        "aux_code_keep_empty",
        "shuangpin_raw_preedit",
        "shuangpin_raw_window",
        "shuangpin_full_pinyin",
        "hold_selections",
        "input_log",
        "learning",
        "system_text_replacements",
    ];
    const ENUM_KEYS: [(&str, &[&str]); 8] = [
        ("layout", &["vertical", "horizontal"]),
        ("renderer", &["qingjian", "system"]),
        ("preedit", &["both", "inline", "window"]),
        ("shift_letter", &["passthrough", "compose"]),
        ("numpad_digit", &["select", "direct"]),
        ("double_shift_input", &["off", "vietnamese"]),
        ("key_hint", &["off", "auto", "always"]),
        ("log_level", &["info", "debug"]),
    ];
    const INT_KEYS: [(&str, u32, u32); 6] = [
        ("page_size", 1, 9),
        ("english_max_in_chinese", 1, 4),
        ("english_min_position", 1, 9),
        ("candidate_font_weight", 0, 900),
        ("key_hint_opacity", 30, 100),
        ("key_hint_size", 60, 150),
    ];

    // 固定种子的 LCG：同一轮次在任何机器上生成同一组用例。
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let next = |state: &mut u64| -> u32 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (*state >> 33) as u32
    };

    for round in 0..40u32 {
        let mut lines = Vec::new();
        let mut used: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let bool_count = next(&mut state) as usize % 4;
        for _ in 0..bool_count {
            let key = BOOL_KEYS[next(&mut state) as usize % BOOL_KEYS.len()];
            if used.insert(key) {
                let value = next(&mut state) % 2 == 0;
                lines.push(format!("{key} = {value}"));
            }
        }
        let enum_count = next(&mut state) as usize % 3;
        for _ in 0..enum_count {
            let (key, options) = ENUM_KEYS[next(&mut state) as usize % ENUM_KEYS.len()];
            if used.insert(key) {
                let value = options[next(&mut state) as usize % options.len()];
                lines.push(format!("{key} = \"{value}\""));
            }
        }
        let int_count = next(&mut state) as usize % 3;
        for _ in 0..int_count {
            let (key, lo, hi) = INT_KEYS[next(&mut state) as usize % INT_KEYS.len()];
            if used.insert(key) {
                let value = lo + next(&mut state) % (hi - lo + 1);
                lines.push(format!("{key} = {value}"));
            }
        }
        let source = lines.join("\n");
        let first: GeneralConfig = toml::from_str(&source).unwrap();
        let serialized = toml::to_string(&first).unwrap();
        let second: GeneralConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(first, second, "第 {round} 轮 roundtrip 不一致：{source}");
    }
}
