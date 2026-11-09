//! 肤色：带 Unicode `Emoji_Modifier_Base` 属性的 emoji 可以接五档肤色修饰符（U+1F3FB–1F3FF）。
//! 面板目录只收不带肤色的那一条，壳长按时用 [`skin_tone_variants`] 列出五档给人挑。

/// 带 `Emoji_Modifier_Base` 属性的码位，取自 emoji-data.txt（Emoji 15.1）。
const MODIFIER_BASE: &[(u32, u32)] = &[
    (0x261D, 0x261D),
    (0x26F9, 0x26F9),
    (0x270A, 0x270C),
    (0x270D, 0x270D),
    (0x1F385, 0x1F385),
    (0x1F3C2, 0x1F3C4),
    (0x1F3C7, 0x1F3C7),
    (0x1F3CA, 0x1F3CA),
    (0x1F3CB, 0x1F3CC),
    (0x1F442, 0x1F443),
    (0x1F446, 0x1F450),
    (0x1F466, 0x1F46B),
    (0x1F46C, 0x1F46D),
    (0x1F46E, 0x1F478),
    (0x1F47C, 0x1F47C),
    (0x1F481, 0x1F483),
    (0x1F485, 0x1F487),
    (0x1F48F, 0x1F48F),
    (0x1F491, 0x1F491),
    (0x1F4AA, 0x1F4AA),
    (0x1F574, 0x1F575),
    (0x1F57A, 0x1F57A),
    (0x1F590, 0x1F590),
    (0x1F595, 0x1F596),
    (0x1F645, 0x1F647),
    (0x1F64B, 0x1F64F),
    (0x1F6A3, 0x1F6A3),
    (0x1F6B4, 0x1F6B5),
    (0x1F6B6, 0x1F6B6),
    (0x1F6C0, 0x1F6C0),
    (0x1F6CC, 0x1F6CC),
    (0x1F90C, 0x1F90C),
    (0x1F90F, 0x1F90F),
    (0x1F918, 0x1F918),
    (0x1F919, 0x1F91E),
    (0x1F91F, 0x1F91F),
    (0x1F926, 0x1F926),
    (0x1F930, 0x1F930),
    (0x1F931, 0x1F932),
    (0x1F933, 0x1F939),
    (0x1F93C, 0x1F93E),
    (0x1F977, 0x1F977),
    (0x1F9B5, 0x1F9B6),
    (0x1F9B8, 0x1F9B9),
    (0x1F9BB, 0x1F9BB),
    (0x1F9CD, 0x1F9CF),
    (0x1F9D1, 0x1F9DD),
    (0x1FAC3, 0x1FAC5),
    (0x1FAF0, 0x1FAF6),
    (0x1FAF7, 0x1FAF8),
];

const SKIN_TONES: [char; 5] = [
    '\u{1F3FB}',
    '\u{1F3FC}',
    '\u{1F3FD}',
    '\u{1F3FE}',
    '\u{1F3FF}',
];

fn is_modifier_base(c: char) -> bool {
    let code = u32::from(c);
    MODIFIER_BASE
        .iter()
        .any(|&(start, end)| (start..=end).contains(&code))
}

/// 五档肤色的写法，浅到深；不能接肤色的返回空。
///
/// 修饰符接在第一个字符后面（紧跟的 FE0F 去掉），零宽连接的职业、性别序列（🧑‍💻、👮‍♀️）也是这样。
/// 序列里有不止一个能接肤色的字符（一家人、握手的两只手）时两边要各配一档，组合太多，不给。
pub fn skin_tone_variants(text: &str) -> Vec<String> {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return Vec::new();
    };
    if !is_modifier_base(first) || text.chars().skip(1).any(is_modifier_base) {
        return Vec::new();
    }
    let rest = chars.as_str();
    let rest = rest.strip_prefix('\u{FE0F}').unwrap_or(rest);
    if rest.chars().next().is_some_and(|c| SKIN_TONES.contains(&c)) {
        return Vec::new();
    }
    SKIN_TONES
        .iter()
        .map(|tone| format!("{first}{tone}{rest}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_emoji_gets_five_tones() {
        let tones = skin_tone_variants("👍");
        assert_eq!(tones.len(), 5);
        assert_eq!(tones[0], "👍🏻");
        assert_eq!(tones[4], "👍🏿");
    }

    #[test]
    fn variation_selector_is_dropped() {
        assert_eq!(skin_tone_variants("✌️")[2], "✌🏽");
    }

    #[test]
    fn tone_goes_after_the_first_person_in_a_sequence() {
        assert_eq!(skin_tone_variants("🧑‍💻")[0], "🧑🏻‍💻");
        assert_eq!(skin_tone_variants("👮‍♀️")[1], "👮🏼‍♀️");
    }

    #[test]
    fn no_tones_for_plain_or_multi_person_emoji() {
        for text in ["😂", "🐱", "👨‍👩‍👧", "🧑‍🤝‍🧑", "👍🏻", "", "a"]
        {
            assert!(skin_tone_variants(text).is_empty(), "{text}");
        }
    }
}
