//! 用户自定义的模糊音规则：配置里写成 `音=音`，两边都是声母（`zh=j`）、都是韵母（`ian=iang`）或都是完整音节（`fa=hua`）。

use crate::parser;

/// 声母，双字母的在前：`zhi` 的声母是 `zh` 不是 `z`。
const INITIALS: [&str; 23] = [
    "zh", "ch", "sh", "b", "p", "m", "f", "d", "t", "n", "l", "g", "k", "h", "j", "q", "x", "r",
    "z", "c", "s", "y", "w",
];

/// 韵母（含 `ue` / `ve` 两种写法）。
const FINALS: [&str; 35] = [
    "a", "o", "e", "i", "u", "v", "ai", "ei", "ui", "ao", "ou", "iu", "ie", "ue", "ve", "er", "an",
    "en", "in", "un", "vn", "ang", "eng", "ing", "ong", "ia", "iao", "ian", "iang", "iong", "ua",
    "uo", "uai", "uan", "uang",
];

/// 一条自定义规则，两个方向都换。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomRule {
    /// 声母互换（`zh=j`），完整音节与没打完的音节都生效。
    Initial(String, String),

    /// 韵母互换（`ian=iang`），按结尾换，只对完整音节生效（与内置的 an = ang 一样）。
    Final(String, String),

    /// 整个音节互换（`fa=hua`），只对完整音节生效。
    Syllable(String, String),
}

impl CustomRule {
    /// 读一条 `音=音`（两边去空白、转小写，`ü` 写作 `v`）。两边不同类、相同或认不出时给 `None`。
    /// 两边既是韵母又是音节（`an=ang`）时按韵母算：韵母规则覆盖它所有带声母的音节。
    pub fn parse(text: &str) -> Option<Self> {
        let (left, right) = text.split_once('=')?;
        let normalize = |s: &str| s.trim().to_lowercase().replace('ü', "v");
        let (a, b) = (normalize(left), normalize(right));
        if a.is_empty() || a == b {
            return None;
        }
        let both = |set: &[&str]| set.contains(&a.as_str()) && set.contains(&b.as_str());
        if both(&INITIALS) {
            Some(Self::Initial(a, b))
        } else if both(&FINALS) {
            Some(Self::Final(a, b))
        } else if parser::is_syllable(&a) && parser::is_syllable(&b) {
            Some(Self::Syllable(a, b))
        } else {
            None
        }
    }

    /// 规范写法 `a=b`：存进配置、在设置里显示都用它。
    pub fn text(&self) -> String {
        let (a, b) = self.sides();
        format!("{a}={b}")
    }

    /// 对一种写法套一次这条规则得到的写法（没对上就是空的；合不合法由调用方过滤）。
    pub(super) fn apply(&self, form: &str, complete: bool, out: &mut Vec<String>) {
        let (a, b) = self.sides();
        match self {
            Self::Initial(..) => {
                let initial = initial_of(form);
                let rest = &form[initial.len()..];
                if initial == a {
                    out.push(format!("{b}{rest}"));
                } else if initial == b {
                    out.push(format!("{a}{rest}"));
                }
            }
            Self::Final(..) if complete => {
                let rest = &form[initial_of(form).len()..];
                for (from, to) in [(a, b), (b, a)] {
                    if let Some(head) = rest.strip_suffix(from) {
                        out.push(format!("{}{head}{to}", initial_of(form)));
                    }
                }
            }
            Self::Syllable(..) if complete => {
                if form == a {
                    out.push(b.to_owned());
                } else if form == b {
                    out.push(a.to_owned());
                }
            }
            _ => {}
        }
    }

    fn sides(&self) -> (&str, &str) {
        match self {
            Self::Initial(a, b) | Self::Final(a, b) | Self::Syllable(a, b) => (a, b),
        }
    }
}

/// 一种写法的声母（零声母给空串）。
fn initial_of(form: &str) -> &str {
    INITIALS
        .iter()
        .find(|initial| form.starts_with(**initial))
        .copied()
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn applied(rule: &str, form: &str, complete: bool) -> Vec<String> {
        let mut out = Vec::new();
        CustomRule::parse(rule)
            .unwrap()
            .apply(form, complete, &mut out);
        out
    }

    #[test]
    fn parses_by_kind() {
        assert_eq!(
            CustomRule::parse(" ZH = j "),
            Some(CustomRule::Initial("zh".into(), "j".into()))
        );
        assert_eq!(
            CustomRule::parse("ian=iang"),
            Some(CustomRule::Final("ian".into(), "iang".into()))
        );
        assert_eq!(
            CustomRule::parse("fa=hua"),
            Some(CustomRule::Syllable("fa".into(), "hua".into()))
        );
        assert_eq!(
            CustomRule::parse("lü=lu"),
            Some(CustomRule::Syllable("lv".into(), "lu".into()))
        );
        // 两边不同类、一样、写错的不认
        for bad in ["zh=ang", "an=an", "eng", "xx=yy", "=a"] {
            assert_eq!(CustomRule::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn initials_do_not_split_two_letter_ones() {
        assert_eq!(applied("z=j", "zhi", true), Vec::<String>::new());
        assert_eq!(applied("z=j", "zi", true), ["ji"]);
        assert_eq!(applied("zh=j", "ji", true), ["zhi"]);
        // 没打完的音节也换声母
        assert_eq!(applied("zh=j", "zh", false), ["j"]);
    }

    #[test]
    fn finals_replace_the_ending_after_the_initial() {
        assert_eq!(applied("ian=iang", "xian", true), ["xiang"]);
        assert_eq!(applied("ian=iang", "xiang", true), ["xian"]);
        assert_eq!(applied("eng=ong", "zhong", true), ["zheng"]);
        // 没打完的音节不套韵母规则
        assert_eq!(applied("ian=iang", "xian", false), Vec::<String>::new());
    }

    #[test]
    fn syllables_swap_whole() {
        assert_eq!(applied("fa=hua", "hua", true), ["fa"]);
        assert_eq!(applied("fa=hua", "fan", true), Vec::<String>::new());
    }
}
