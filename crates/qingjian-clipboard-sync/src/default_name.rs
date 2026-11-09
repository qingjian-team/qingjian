//! 没起名时的默认设备名：「形容词的角色」。由公钥决定，同一台设备每次都一样。
//! 名字只给人看，撞了也不影响同步，设备靠公钥区分；界面在重名时补 id 尾巴（[`short_id`]）。

const ADJECTIVES: [(&str, &str); 12] = [
    ("坚强的", "Brave"),
    ("安静的", "Quiet"),
    ("闪亮的", "Shining"),
    ("认真的", "Earnest"),
    ("温柔的", "Gentle"),
    ("勇敢的", "Bold"),
    ("害羞的", "Shy"),
    ("元气的", "Lively"),
    ("迷路的", "Wandering"),
    ("熬夜的", "Sleepless"),
    ("倔强的", "Stubborn"),
    ("自由的", "Free"),
];

/// 《孤独摇滚！》《BanG Dream! It's MyGO!!!!!》《BanG Dream! Ave Mujica》的角色。
const CHARACTERS: [(&str, &str); 14] = [
    ("后藤一里", "Hitori Gotoh"),
    ("伊地知虹夏", "Nijika Ijichi"),
    ("山田凉", "Ryo Yamada"),
    ("喜多郁代", "Ikuyo Kita"),
    ("高松灯", "Tomori Takamatsu"),
    ("千早爱音", "Anon Chihaya"),
    ("要乐奈", "Rana Kaname"),
    ("长崎爽世", "Soyo Nagasaki"),
    ("椎名立希", "Taki Shiina"),
    ("丰川祥子", "Sakiko Togawa"),
    ("三角初华", "Uika Misumi"),
    ("若叶睦", "Mutsumi Wakaba"),
    ("八幡海铃", "Umiri Yahata"),
    ("祐天寺若麦", "Nyamu Yutenji"),
];

/// 默认设备名。界面语言不是中文时用英文写法。
pub fn default_device_name(public: &[u8; 32], chinese: bool) -> String {
    let adjective = ADJECTIVES[usize::from(public[0]) % ADJECTIVES.len()];
    let character = CHARACTERS[usize::from(public[1]) % CHARACTERS.len()];
    if chinese {
        format!("{}{}", adjective.0, character.0)
    } else {
        format!("{} {}", adjective.1, character.1)
    }
}

/// 设备 id 的尾巴，重名时显示在名字后面（`#a1b2`）。
pub fn short_id(device_id: &str) -> &str {
    device_id
        .get(device_id.len().saturating_sub(4)..)
        .unwrap_or(device_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_is_stable_and_follows_the_language() {
        let key = [3_u8; 32];
        assert_eq!(
            default_device_name(&key, true),
            default_device_name(&key, true)
        );
        assert_eq!(default_device_name(&key, true), "认真的喜多郁代");
        assert_eq!(default_device_name(&key, false), "Earnest Ikuyo Kita");
    }

    #[test]
    fn short_id_takes_the_tail() {
        assert_eq!(short_id("abcdef"), "cdef");
        assert_eq!(short_id("ab"), "ab");
    }
}
