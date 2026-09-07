use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::table::{self, DIGRAPH_INITIALS, Table};
use super::{Decoded, Unit};
use crate::parser;

/// 双拼方案。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scheme {
    /// 小鹤双拼。
    Xiaohe,

    /// 自然码。
    Ziranma,

    /// 微软双拼。
    Microsoft,

    /// 搜狗双拼。
    Sogou,
}

impl Scheme {
    /// 全部方案，设置界面按这个顺序列出。
    pub const ALL: [Self; 4] = [Self::Xiaohe, Self::Ziranma, Self::Microsoft, Self::Sogou];

    /// 配置文件里的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Xiaohe => "xiaohe",
            Self::Ziranma => "ziranma",
            Self::Microsoft => "microsoft",
            Self::Sogou => "sogou",
        }
    }

    /// 界面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Xiaohe => "小鹤双拼",
            Self::Ziranma => "自然码",
            Self::Microsoft => "微软双拼",
            Self::Sogou => "搜狗双拼",
        }
    }

    fn table(self) -> &'static Table {
        match self {
            Self::Xiaohe => &table::XIAOHE,
            Self::Ziranma => &table::ZIRANMA,
            Self::Microsoft => &table::MICROSOFT,
            Self::Sogou => &table::SOGOU,
        }
    }

    /// 这套方案是否用到 `;` 键。
    pub fn uses_semicolon(self) -> bool {
        self.table().semicolon
    }

    /// `c` 是不是这套方案的键：小写字母，微软 / 搜狗再加 `;`。
    pub fn is_key(self, c: char) -> bool {
        c.is_ascii_lowercase() || (c == ';' && self.uses_semicolon())
    }

    /// 键 `key` 当声母时是什么：`v` `i` `u` 是 zh ch sh，其他辅音（含 y w）是自己，元音键与 `;` 不是声母。
    pub fn initial(self, key: char) -> Option<&'static str> {
        if let Some((_, initial)) = DIGRAPH_INITIALS.iter().find(|(k, _)| *k == key) {
            return Some(initial);
        }
        parser::INITIALS
            .iter()
            .copied()
            .find(|initial| initial.len() == 1 && initial.starts_with(key))
    }

    /// 键 `key` 当韵母时的候选韵母（按优先级）。
    pub fn finals(self, key: char) -> &'static [&'static str] {
        self.table()
            .finals
            .iter()
            .find(|(k, _)| *k == key)
            .map_or(&[], |(_, finals)| finals)
    }

    /// 两个键拼成的音节；拼不出合法音节时为 `None`。
    pub fn syllable(self, first: char, second: char) -> Option<String> {
        let pair = [first, second];
        for (syllable, spellings) in self.table().zero_initials {
            if spellings.iter().any(|s| s.chars().eq(pair.iter().copied())) {
                return Some((*syllable).to_owned());
            }
        }
        let initial = self.initial(first)?;
        self.finals(second).iter().find_map(|final_| {
            let syllable = format!("{initial}{final_}");
            parser::is_syllable(&syllable).then_some(syllable)
        })
    }

    /// 一个全拼音节的主写法（两个键）。测试与文档用；拆成声母 + 韵母后查表，零声母查零声母表。
    pub fn encode(self, syllable: &str) -> Option<[char; 2]> {
        let table = self.table();
        if let Some((_, spellings)) = table.zero_initials.iter().find(|(s, _)| *s == syllable) {
            let mut chars = spellings[0].chars();
            return Some([chars.next()?, chars.next()?]);
        }
        let initial = parser::INITIALS
            .iter()
            .filter(|initial| syllable.starts_with(*initial))
            .max_by_key(|initial| initial.len())?;
        let final_ = &syllable[initial.len()..];
        let first = DIGRAPH_INITIALS
            .iter()
            .find(|(_, i)| i == initial)
            .map(|(key, _)| *key)
            .or_else(|| initial.chars().next())?;
        let second = table
            .finals
            .iter()
            .find(|(_, finals)| finals.contains(&final_))
            .map(|(key, _)| *key)?;
        Some([first, second])
    }

    /// 把敲的键翻成全拼。两键一组从左到右配对；配不出合法音节的位置起全部原样留作尾巴；
    /// 末尾落单的一键当声母（或元音）前缀；用户自己敲的 `'` 结束当前配对。
    pub fn decode(self, keys: &str) -> Decoded {
        let chars: Vec<char> = keys.chars().collect();
        let mut units = Vec::with_capacity(chars.len() / 2 + 1);
        let mut index = 0;
        while index < chars.len() {
            let first = chars[index];
            if first == '\'' {
                units.push(Unit::separator());
                index += 1;
                continue;
            }
            let second = chars.get(index + 1).copied().filter(|c| *c != '\'');
            let unit = match second {
                Some(second) => self.syllable(first, second).map(|pinyin| Unit {
                    keys: [first, second].iter().collect(),
                    pinyin,
                    complete: true,
                }),
                None => self.partial(first).map(|pinyin| Unit {
                    keys: first.to_string(),
                    pinyin,
                    complete: false,
                }),
            };
            let Some(unit) = unit else {
                break;
            };
            index += unit.keys.len();
            units.push(unit);
        }
        Decoded::new(units, chars[index..].iter().collect())
    }

    /// 落单的一键代表的前缀：声母键是声母，元音键是元音本身（`a` 后面可能是 ai / an / ang / ao）。
    fn partial(self, key: char) -> Option<String> {
        if let Some(initial) = self.initial(key) {
            return Some(initial.to_owned());
        }
        matches!(key, 'a' | 'e' | 'o').then(|| key.to_string())
    }
}

impl FromStr for Scheme {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|scheme| scheme.key() == text.trim().to_ascii_lowercase())
            .ok_or_else(|| format!("unknown shuangpin scheme: {text:?}"))
    }
}

impl fmt::Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 每套方案：全部音节都能编成两键、再解回来是同一个音节。
    /// 只有 `lue` / `nue`（与 `lve` / `nve` 同键）和 `lo`（与 `luo` 同键）解回来是另一个写法。
    #[test]
    fn every_syllable_round_trips() {
        for scheme in Scheme::ALL {
            for syllable in parser::SYLLABLES {
                let keys = scheme
                    .encode(syllable)
                    .unwrap_or_else(|| panic!("{scheme}: {syllable} 编不出"));
                let decoded = scheme
                    .syllable(keys[0], keys[1])
                    .unwrap_or_else(|| panic!("{scheme}: {syllable} → {keys:?} 解不回"));
                let expected = match *syllable {
                    "lue" => "lve",
                    "nue" => "nve",
                    "lo" => "luo",
                    other => other,
                };
                assert_eq!(decoded, expected, "{scheme}: {syllable} → {keys:?}");
            }
        }
    }

    /// 两键组合最多对应一个音节：解码不会有歧义。
    #[test]
    fn every_key_pair_is_unambiguous() {
        for scheme in Scheme::ALL {
            let keys: Vec<char> = "abcdefghijklmnopqrstuvwxyz;".chars().collect();
            for first in &keys {
                for second in &keys {
                    // `syllable` 本身就是确定的；这里确认零声母写法与「声母 + 韵母」不撞车
                    let zero = scheme
                        .table()
                        .zero_initials
                        .iter()
                        .filter(|(_, spellings)| {
                            spellings.iter().any(|s| s.chars().eq([*first, *second]))
                        })
                        .count();
                    assert!(zero <= 1, "{scheme}: {first}{second} 对应多个零声母音节");
                    if zero == 1 {
                        assert!(
                            scheme.initial(*first).is_none(),
                            "{scheme}: {first}{second} 既是零声母写法又能当声母开头"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn well_known_words_decode_per_scheme() {
        let cases = [
            (Scheme::Xiaohe, "nihc", "ni'hao"),
            (Scheme::Xiaohe, "vsgo", "zhong'guo"),
            (Scheme::Xiaohe, "ulpb", "shuang'pin"),
            (Scheme::Xiaohe, "xtxi", "xue'xi"),
            (Scheme::Xiaohe, "uiui", "shi'shi"),
            (Scheme::Xiaohe, "wdgo", "wai'guo"),
            (Scheme::Xiaohe, "yske", "yong'ke"),
            (Scheme::Xiaohe, "nvhd", "nv'hai"),
            (Scheme::Xiaohe, "ahnv", "ang'nv"),
            (Scheme::Ziranma, "nihk", "ni'hao"),
            (Scheme::Ziranma, "udpn", "shuang'pin"),
            (Scheme::Ziranma, "xtxi", "xue'xi"),
            (Scheme::Ziranma, "eeyu", "e'yu"),
            (Scheme::Microsoft, "nihk", "ni'hao"),
            (Scheme::Microsoft, "udpn", "shuang'pin"),
            (Scheme::Microsoft, "ly", "lv"),
            (Scheme::Microsoft, "oaol", "a'ai"),
            (Scheme::Microsoft, "lv", "lve"),
            (Scheme::Microsoft, "x;", "xing"),
            (Scheme::Sogou, "nihk", "ni'hao"),
            (Scheme::Sogou, "oe", "e"),
            (Scheme::Sogou, "y;", "ying"),
        ];
        for (scheme, keys, expected) in cases {
            assert_eq!(scheme.decode(keys).pinyin(), expected, "{scheme}: {keys}");
        }
        // 搜狗的 v 不兼作 üe
        assert_eq!(Scheme::Sogou.decode("lv").pinyin(), "");
        assert_eq!(Scheme::Sogou.decode("lv").tail(), "lv");
    }

    #[test]
    fn trailing_single_key_is_a_prefix() {
        let decoded = Scheme::Xiaohe.decode("kdf");
        assert_eq!(decoded.pinyin(), "kai'f");
        assert!(!decoded.is_complete());
        assert_eq!(Scheme::Xiaohe.decode("v").pinyin(), "zh");
        assert_eq!(Scheme::Xiaohe.decode("a").pinyin(), "a");
        // `;` 落单不是任何东西
        assert_eq!(Scheme::Microsoft.decode(";").pinyin(), "");
        assert_eq!(Scheme::Microsoft.decode(";").tail(), ";");
    }

    #[test]
    fn invalid_pair_starts_the_tail() {
        let decoded = Scheme::Xiaohe.decode("nibl");
        assert_eq!(decoded.pinyin(), "ni");
        assert_eq!(decoded.tail(), "bl");
        assert_eq!(decoded.marked(), "ni'bl");
    }

    #[test]
    fn apostrophe_ends_a_pair() {
        let decoded = Scheme::Xiaohe.decode("x'ni");
        assert_eq!(decoded.pinyin(), "x'ni");
        assert_eq!(decoded.units()[1].keys, "'");
        assert_eq!(Scheme::Xiaohe.decode("ni'hc").pinyin(), "ni'hao");
    }

    #[test]
    fn parses_config_keys() {
        for scheme in Scheme::ALL {
            assert_eq!(scheme.key().parse::<Scheme>().unwrap(), scheme);
        }
        assert_eq!("Xiaohe ".parse::<Scheme>().unwrap(), Scheme::Xiaohe);
        assert!("flypy".parse::<Scheme>().is_err());
        assert!(Scheme::Microsoft.uses_semicolon());
        assert!(!Scheme::Xiaohe.uses_semicolon());
    }
}
