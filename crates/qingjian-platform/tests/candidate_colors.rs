//! 新增配色配置保持旧配置兼容，并能独立持久化与恢复默认。

use qingjian_platform::Config;

#[test]
fn old_configs_keep_default_colors_and_existing_typography() {
    let config: Config = toml::from_str(
        "[general]\nfont = 'Helvetica'\ncandidate_font_size = 18\nannotation_bold = true\n",
    )
    .unwrap();
    assert!(config.general.candidate_background_color.is_empty());
    assert!(config.general.candidate_text_color.is_empty());
    assert!(config.general.candidate_pos_color.is_empty());
    assert!(config.general.candidate_word_color.is_empty());
    assert!(config.general.candidate_fresh_word_color.is_empty());
    assert!(config.general.candidate_highlight_color.is_empty());
    assert_eq!(config.general.font, "Helvetica");
    assert_eq!(config.general.candidate_font_size, 18);
    assert!(config.general.annotation_bold);
}

#[test]
fn six_colors_survive_round_trip_and_reset_independently() {
    let mut config: Config = toml::from_str(
        "[general]\ncandidate_background_color = '#F4F1EA'\ncandidate_text_color = '#243449'\ncandidate_pos_color = '#8C5E35'\ncandidate_word_color = '#23635B'\ncandidate_highlight_color = '#D8E8D0'\n",
    ).unwrap();
    // 旧版五项颜色不变，新版生词色默认独立。
    assert!(config.general.candidate_fresh_word_color.is_empty());
    config.general.candidate_fresh_word_color = "#8844CC80".into();
    let saved = toml::to_string(&config).unwrap();
    assert_eq!(toml::from_str::<Config>(&saved).unwrap(), config);
    config.general.candidate_word_color.clear();
    let reset: Config = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
    assert!(reset.general.candidate_word_color.is_empty());
    assert_eq!(reset.general.candidate_background_color, "#F4F1EA");
    assert_eq!(reset.general.candidate_text_color, "#243449");
    assert_eq!(reset.general.candidate_pos_color, "#8C5E35");
    assert_eq!(reset.general.candidate_highlight_color, "#D8E8D0");
    assert_eq!(reset.general.candidate_fresh_word_color, "#8844CC80");
    config.general.candidate_word_color = "#23635B".into();
    config.general.candidate_fresh_word_color.clear();
    let reset: Config = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
    assert!(reset.general.candidate_fresh_word_color.is_empty());
    assert_eq!(reset.general.candidate_word_color, "#23635B");
}
