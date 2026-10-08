//! 独立词性字号兼容已有配置，不改动字体、字号、加粗或自定义配色。

use qingjian_platform::Config;

#[test]
fn pos_defaults_and_round_trips_without_changing_existing_preferences() {
    let mut config: Config = toml::from_str("[general]\nfont='BM Dohyeon'\ncandidate_font='Source Han Sans SC'\ncandidate_font_size=18\nannotation_font_size=20\ncandidate_bold=true\ncandidate_word_color='#F56A00'\n").unwrap();
    assert_eq!(config.general.pos_font_size, 12);
    let old = config.clone();
    config.general.pos_font_size = 10;
    let mut reloaded: Config = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
    assert_eq!(reloaded.general.pos_font_size, 10);
    reloaded.general.pos_font_size = old.general.pos_font_size;
    assert_eq!(reloaded, old);
}
