use std::collections::HashSet;

use qingjian_dictionary::{SyllablePattern, canonical_syllable};

use super::table::Table;
use super::{Decoded, Unit};
use crate::fuzzy::FuzzyRules;
use crate::parser;

/// 能当双拼键的字符：小写字母与 `;`（微软 / 搜狗 / 紫光的 ing）。
fn is_layout_key(c: char) -> bool {
    c.is_ascii_lowercase() || c == ';'
}

/// 一套双拼方案的键位：内置九套（见 [`super::Scheme`]）与用户方案文件夹里写的是同一个结构、同一套解码。
///
/// 声母键只列翘舌声母（键 → zh / ch / sh），其余辅音键（含 `y` `w`）就是自己；
/// 韵母键 → 可能的韵母，按优先级排（同一键配同一声母能拼出两个合法音节时取前面的，如 `lve` 先于 `lue`）；
/// 零声母音节 → 两键写法，第一种是主写法。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    key: String,
    pub(super) initials: Vec<(char, String)>,
    pub(super) finals: Vec<(char, Vec<String>)>,
    pub(super) zero_initials: Vec<(String, Vec<String>)>,
}

impl Layout {
    /// 空白键位表：`key` 是配置与输入日志里的写法。
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            initials: Vec::new(),
            finals: Vec::new(),
            zero_initials: Vec::new(),
        }
    }

    pub(super) fn from_table(key: &str, table: &Table) -> Self {
        Self {
            key: key.to_owned(),
            initials: table
                .digraph_initials
                .iter()
                .map(|(key, initial)| (*key, (*initial).to_owned()))
                .collect(),
            finals: table
                .finals
                .iter()
                .map(|(key, finals)| (*key, finals.iter().map(|f| (*f).to_owned()).collect()))
                .collect(),
            zero_initials: table
                .zero_initials
                .iter()
                .map(|(syllable, spellings)| {
                    (
                        (*syllable).to_owned(),
                        spellings.iter().map(|s| (*s).to_owned()).collect(),
                    )
                })
                .collect(),
        }
    }

    /// 同一份键位换个写法（拿内置方案当底子改时用）。
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = key.into();
        self
    }

    /// 配置与输入日志里的写法。
    pub fn key(&self) -> &str {
        &self.key
    }

    /// 让键 `key` 当翘舌声母 `initial`；`None` 撤掉。
    pub fn set_initial(&mut self, key: char, initial: Option<&str>) {
        self.initials.retain(|(k, _)| *k != key);
        if let Some(initial) = initial {
            self.initials.push((key, initial.to_owned()));
        }
    }

    /// 键 `key` 上的韵母整个换成 `finals`（按优先级）；空的就是撤掉。
    pub fn set_finals(&mut self, key: char, finals: Vec<String>) {
        match self.finals.iter_mut().find(|(k, _)| *k == key) {
            Some((_, slot)) => *slot = finals,
            None => self.finals.push((key, finals)),
        }
        self.finals.retain(|(_, finals)| !finals.is_empty());
    }

    /// 零声母音节 `syllable` 的写法整个换成 `spellings`（第一种是主写法）；空的就是撤掉。
    pub fn set_zero_initial(&mut self, syllable: &str, spellings: Vec<String>) {
        match self.zero_initials.iter_mut().find(|(s, _)| s == syllable) {
            Some((_, slot)) => *slot = spellings,
            None => self.zero_initials.push((syllable.to_owned(), spellings)),
        }
        self.zero_initials
            .retain(|(_, spellings)| !spellings.is_empty());
    }

    /// 检查键位表写得对不对：键只能是小写字母或 `;`，声母键只能放 zh / ch / sh 且不能占别的声母本身的键，
    /// 韵母要是普通话里有的韵母，零声母写法正好两个键。`;` 只能当第二个键：
    /// 壳只在前面有落单的声母时才把它收进缓冲区，打头敲是标点（见 `Engine::takes_semicolon`）。
    /// 写对了返回打不出来的音节：没有哪两个键解得出它（只提醒，不算错：有的方案本来就让生僻音节让位）。
    pub fn check(&self) -> Result<Vec<&'static str>, String> {
        for (key, initial) in &self.initials {
            if !key.is_ascii_lowercase() {
                return Err(format!("声母键 {key:?} 不是小写字母"));
            }
            if !matches!(initial.as_str(), "zh" | "ch" | "sh") {
                return Err(format!("{key} 键上的声母 {initial:?} 不是 zh / ch / sh"));
            }
            if parser::INITIALS.contains(&key.to_string().as_str()) {
                return Err(format!("{key} 本身就是声母，不能再放 {initial}"));
            }
        }
        for (key, finals) in &self.finals {
            if !is_layout_key(*key) {
                return Err(format!("韵母键 {key:?} 不是小写字母或 ;"));
            }
            if let Some(final_) = finals.iter().find(|final_| !is_final(final_)) {
                return Err(format!("{key} 键上的 {final_:?} 不是韵母"));
            }
        }
        for (syllable, spellings) in &self.zero_initials {
            if !parser::is_syllable(syllable) {
                return Err(format!("零声母 {syllable:?} 不是音节"));
            }
            if let Some(spelling) = spellings.iter().find(|s| {
                let mut chars = s.chars();
                !matches!(
                    (chars.next(), chars.next(), chars.next()),
                    (Some(first), Some(second), None)
                        if first.is_ascii_lowercase() && is_layout_key(second)
                )
            }) {
                return Err(format!(
                    "{syllable} 的写法 {spelling:?} 不是两个键（; 只能放第二个）"
                ));
            }
        }
        let keys: Vec<char> = ('a'..='z').chain([';']).collect();
        let reachable: HashSet<String> = keys
            .iter()
            .flat_map(|first| keys.iter().map(move |second| (*first, *second)))
            .filter_map(|(first, second)| self.syllable(first, second))
            .collect();
        Ok(parser::SYLLABLES
            .iter()
            .copied()
            .filter(|syllable| {
                // lo（咯）各方案都与 luo 同键，解出来是 luo
                let decoded = match *syllable {
                    "lo" => "luo",
                    other => canonical_syllable(other),
                };
                !reachable.contains(decoded)
            })
            .collect())
    }

    /// 这套方案是否用到 `;` 键。
    pub fn uses_semicolon(&self) -> bool {
        self.initials.iter().any(|(key, _)| *key == ';')
            || self.finals.iter().any(|(key, _)| *key == ';')
            || self
                .zero_initials
                .iter()
                .any(|(_, spellings)| spellings.iter().any(|s| s.contains(';')))
    }

    /// `c` 是不是这套方案的键：小写字母，用到 `;` 的方案再加 `;`。
    pub fn is_key(&self, c: char) -> bool {
        c.is_ascii_lowercase() || (c == ';' && self.uses_semicolon())
    }

    /// 键 `key` 当声母时是什么：翘舌声母按方案映射，其他辅音（含 y w）是自己，元音键与 `;` 不是声母。
    pub fn initial(&self, key: char) -> Option<&str> {
        if let Some((_, initial)) = self.initials.iter().find(|(k, _)| *k == key) {
            return Some(initial);
        }
        parser::INITIALS
            .iter()
            .copied()
            .find(|initial| initial.len() == 1 && initial.starts_with(key))
    }

    /// 键 `key` 当韵母时的候选韵母（按优先级）。
    pub fn finals(&self, key: char) -> &[String] {
        self.finals
            .iter()
            .find(|(k, _)| *k == key)
            .map_or(&[], |(_, finals)| finals)
    }

    /// 两个键拼成的音节；拼不出合法音节时为 `None`。
    pub fn syllable(&self, first: char, second: char) -> Option<String> {
        let pair = [first, second];
        for (syllable, spellings) in &self.zero_initials {
            if spellings.iter().any(|s| s.chars().eq(pair.iter().copied())) {
                return Some(syllable.clone());
            }
        }
        let initial = self.initial(first)?;
        // jv / qv 这类全拼键盘别名也是合法音节，解码结果统一成词库写法
        let valid: Vec<String> = self
            .finals(second)
            .iter()
            .map(|final_| format!("{initial}{final_}"))
            .filter(|syllable| parser::is_syllable(syllable))
            .collect();
        valid
            .iter()
            .find(|syllable| !parser::RARE_SYLLABLES.contains(&syllable.as_str()))
            .or(valid.first())
            .map(|syllable| canonical_syllable(syllable).to_owned())
            .or_else(|| {
                // ü 在 j / q / x / y 后写作 u；同键已有的合法音节仍优先。
                (matches!(initial, "j" | "q" | "x" | "y")
                    && self.finals(second).iter().any(|f| f == "v"))
                .then(|| format!("{initial}u"))
            })
    }

    /// 一个全拼音节的主写法（两个键）。测试与文档用；拆成声母 + 韵母后查表，零声母查零声母表。
    ///
    /// 入参接受全拼键盘别名（`jv`、`jve` 等）。先按原写法编码，保留首道等方案里
    /// `lue`/`nue` 与 `lve`/`nve` 的故意分键；原写法查不到键位时（如 j/q/x/y 后的
    /// `van`/`vn`，#531），再归一到主写法重试。
    pub fn encode(&self, syllable: &str) -> Option<[char; 2]> {
        self.encode_spelling(syllable)
            .or_else(|| self.encode_spelling(canonical_syllable(syllable)))
    }

    /// 按给定拼写查两键编码，不做键盘别名归一。
    fn encode_spelling(&self, syllable: &str) -> Option<[char; 2]> {
        if let Some((_, spellings)) = self.zero_initials.iter().find(|(s, _)| s == syllable) {
            let mut chars = spellings.first()?.chars();
            return Some([chars.next()?, chars.next()?]);
        }
        let initial = parser::INITIALS
            .iter()
            .filter(|initial| syllable.starts_with(*initial))
            .max_by_key(|initial| initial.len())?;
        let final_ = &syllable[initial.len()..];
        let first = self
            .initials
            .iter()
            .find(|(_, i)| i == initial)
            .map(|(key, _)| *key)
            .or_else(|| initial.chars().next())?;
        let second = self
            .finals
            .iter()
            .find(|(_, finals)| finals.iter().any(|f| f == final_))
            .map(|(key, _)| *key)?;
        Some([first, second])
    }

    /// 把敲的键翻成全拼。两键一组从左到右配对；配不出合法音节的位置起全部原样留作尾巴；
    /// 末尾落单的一键当声母（或元音）前缀；用户自己敲的 `'` 结束当前配对。
    pub fn decode(&self, keys: &str) -> Decoded {
        self.decode_fuzzy(keys, FuzzyRules::default())
    }

    /// 模糊音能补成合法音节时保留原读音，供查询展开与上屏消耗共用。
    /// 普通解码优先，关闭模糊音时行为与 [`Self::decode`] 一致。
    pub(crate) fn decode_fuzzy(&self, keys: &str, fuzzy: FuzzyRules) -> Decoded {
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
                Some(second) => self
                    .syllable(first, second)
                    .or_else(|| {
                        if !fuzzy.any() {
                            return None;
                        }
                        let initial = self.initial(first)?;
                        self.finals(second).iter().find_map(|final_| {
                            let raw = format!("{initial}{final_}");
                            let expanded = fuzzy.expand(&[SyllablePattern::complete(&raw)]);
                            expanded.has_alternatives().then_some(raw)
                        })
                    })
                    .map(|pinyin| Unit {
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
        // 整串一个音节都配不出来时，再按「每键一个声母」的简拼读一次（`im` → sh + m → 什么）。
        // 能正常解出音节的输入不走这条，完整双拼码的读法不受影响。
        if units.is_empty()
            && !chars.is_empty()
            && let Some(abbreviated) = self.decode_initials(&chars)
        {
            return Decoded::new(abbreviated, String::new());
        }
        Decoded::new(units, chars[index..].iter().collect())
    }

    /// 简拼读法：每键当一个声母（`im` → `sh'm`）。有一个键不是声母键（元音、`'` 之外解不出的）就不是简拼。
    fn decode_initials(&self, chars: &[char]) -> Option<Vec<Unit>> {
        let mut units = Vec::with_capacity(chars.len());
        for &key in chars {
            if key == '\'' {
                units.push(Unit::separator());
                continue;
            }
            let initial = self.initial(key)?;
            units.push(Unit {
                keys: key.to_string(),
                pinyin: initial.to_owned(),
                complete: false,
            });
        }
        Some(units)
    }

    /// 落单的一键代表的前缀：声母键是声母；`a` `e` `o` 是元音本身（`a` 后面可能是 ai / an / ang / ao）；
    /// 别的键看零声母写法拿它打头的音节，都是同一个元音开头就是那个元音（小浪、首道的 `e` 键是翘舌声母，
    /// 零声母 e 一族改用 `u` 引导，`u` 落单就是 `e`）。
    pub(super) fn partial(&self, key: char) -> Option<String> {
        if let Some(initial) = self.initial(key) {
            return Some(initial.to_owned());
        }
        if matches!(key, 'a' | 'e' | 'o') {
            return Some(key.to_string());
        }
        let mut leads = self
            .zero_initials
            .iter()
            .filter(|(_, spellings)| spellings.iter().any(|s| s.starts_with(key)))
            .filter_map(|(syllable, _)| syllable.chars().next());
        let lead = leads.next()?;
        leads.all(|c| c == lead).then(|| lead.to_string())
    }
}

/// 普通话里有的韵母：自己成音节，或接在某个声母后成音节（`v` `ve` 这类全拼键盘写法也算）。
fn is_final(final_: &str) -> bool {
    !final_.is_empty()
        && (parser::is_syllable(final_)
            || parser::INITIALS
                .iter()
                .any(|initial| parser::is_syllable(&format!("{initial}{final_}"))))
}

#[cfg(test)]
mod tests {
    use crate::shuangpin::Scheme;

    /// 内置方案都写得对；打不出来的只有方案自己让位的那几个（与 `every_syllable_round_trips` 的例外一致）。
    #[test]
    fn every_builtin_layout_passes_the_check() {
        for scheme in Scheme::ALL {
            let expected: &[&str] = match scheme {
                Scheme::Xiaolang => &["eng", "biang", "dia", "nen", "lia"],
                Scheme::Shoudao => &["dia"],
                _ => &[],
            };
            assert_eq!(scheme.layout().check().unwrap(), expected, "{scheme}");
        }
    }

    #[test]
    fn a_layout_based_on_a_builtin_can_move_keys() {
        let mut layout = Scheme::Xiaohe.layout().clone().with_key("mine");
        layout.set_finals('k', vec!["ing".to_owned()]);
        layout.set_finals(';', vec!["uai".to_owned()]);
        layout.set_initial('v', None);
        layout.set_initial('o', Some("zh"));
        layout.set_zero_initial("o", vec!["ow".to_owned()]);
        layout.set_zero_initial("ou", vec!["oz".to_owned()]);
        assert_eq!(layout.key(), "mine");
        assert!(layout.uses_semicolon() && layout.is_key(';'));
        assert_eq!(layout.decode("k;").pinyin(), "kuai");
        assert_eq!(layout.decode("os").pinyin(), "zhong");
        assert_eq!(layout.decode("ow").pinyin(), "o");
        assert_eq!(layout.syllable('v', 's'), None);
        // 零声母写法先于声母 + 韵母
        assert_eq!(layout.check().unwrap(), ["zhei", "zhou"]);

        layout.set_finals(';', Vec::new());
        assert_eq!(
            layout.check().unwrap(),
            [
                "guai", "kuai", "huai", "zhei", "zhou", "zhuai", "chuai", "shuai"
            ]
        );
    }

    #[test]
    fn unmatched_keys_fall_back_to_abbreviated_initials() {
        // 一个音节都配不出来时按每键一个声母读（拼音加加 i = sh）
        let decoded = Scheme::Jiajia.decode("im");
        assert_eq!(decoded.pinyin(), "sh'm");
        assert!(!decoded.is_complete());
        assert_eq!(decoded.segmentation().unwrap().to_string(), "sh… m…");
        // 能正常配出音节的输入不受影响
        assert_eq!(Scheme::Jiajia.decode("vege").pinyin(), "zhe'ge");
        // 元音键不是声母键，不按简拼读
        assert!(Scheme::Xiaohe.decode("ae").segmentation().is_none());
    }

    #[test]
    fn the_check_rejects_what_cannot_be_typed() {
        let base = || Scheme::Xiaohe.layout().clone();
        let mut layout = base();
        layout.set_initial('k', Some("zh"));
        assert!(layout.check().unwrap_err().contains("本身就是声母"));
        let mut layout = base();
        layout.set_initial('v', Some("zz"));
        assert!(layout.check().is_err());
        let mut layout = base();
        layout.set_initial(';', Some("zh"));
        assert!(layout.check().is_err());
        let mut layout = base();
        layout.set_zero_initial("ang", vec![";h".to_owned()]);
        assert!(layout.check().is_err());
        let mut layout = base();
        layout.set_finals('1', vec!["ing".to_owned()]);
        assert!(layout.check().is_err());
        let mut layout = base();
        layout.set_finals('k', vec!["ingg".to_owned()]);
        assert!(layout.check().unwrap_err().contains("不是韵母"));
        let mut layout = base();
        layout.set_zero_initial("ang", vec!["a".to_owned()]);
        assert!(layout.check().unwrap_err().contains("两个键"));
        let mut layout = base();
        layout.set_zero_initial("ang", Vec::new());
        assert_eq!(layout.check().unwrap(), ["ang"]);
    }
}
