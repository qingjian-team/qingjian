//! 字符 → macOS 的键码（ANSI 键盘上的位置，Carbon 快捷键与按键状态都认它）。

/// 快捷键的主键（小写字母、数字或句号）对应的键码；别的字符为 `None`。
pub fn key_code(key: char) -> Option<u16> {
    Some(match key.to_ascii_lowercase() {
        'a' => 0,
        's' => 1,
        'd' => 2,
        'f' => 3,
        'h' => 4,
        'g' => 5,
        'z' => 6,
        'x' => 7,
        'c' => 8,
        'v' => 9,
        'b' => 11,
        'q' => 12,
        'w' => 13,
        'e' => 14,
        'r' => 15,
        'y' => 16,
        't' => 17,
        '1' => 18,
        '2' => 19,
        '3' => 20,
        '4' => 21,
        '6' => 22,
        '5' => 23,
        '9' => 25,
        '7' => 26,
        '8' => 28,
        '0' => 29,
        'o' => 31,
        'u' => 32,
        'i' => 34,
        'p' => 35,
        'l' => 37,
        'j' => 38,
        'k' => 40,
        'n' => 45,
        'm' => 46,
        '.' => 47,
        _ => return None,
    })
}

/// `V` 的键码，模拟 ⌘V 用。
pub const V: u16 = 9;

#[cfg(test)]
mod tests {
    use super::key_code;

    #[test]
    fn letters_digits_and_period_have_codes() {
        for key in ('a'..='z').chain('0'..='9').chain(['.']) {
            assert!(key_code(key).is_some(), "{key}");
        }
        assert_eq!(key_code('V'), Some(9));
        assert_eq!(key_code('-'), None);
    }
}
