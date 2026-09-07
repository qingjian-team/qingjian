use qingjian_dictionary::SyllablePattern;
use serde::{Deserialize, Serialize};

use super::Expanded;
use crate::parser;
use crate::ranking::FUZZY_PENALTY;

/// 模糊音开关，配置文件 `[fuzzy]` 分节；缺省全关。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FuzzyRules {
    /// z ↔ zh。
    pub z_zh: bool,

    /// c ↔ ch。
    pub c_ch: bool,

    /// s ↔ sh。
    pub s_sh: bool,

    /// n ↔ l。
    pub n_l: bool,

    /// f ↔ h。
    pub f_h: bool,

    /// l ↔ r。
    pub l_r: bool,

    /// an ↔ ang（含 ian / iang、uan / uang）。
    pub an_ang: bool,

    /// en ↔ eng。
    pub en_eng: bool,

    /// in ↔ ing。
    pub in_ing: bool,
}

impl FuzzyRules {
    /// 全开，测试和 CLI 用。
    pub const ALL: Self = Self {
        z_zh: true,
        c_ch: true,
        s_sh: true,
        n_l: true,
        f_h: true,
        l_r: true,
        an_ang: true,
        en_eng: true,
        in_ing: true,
    };

    pub fn any(&self) -> bool {
        *self != Self::default()
    }

    /// 全部规则的配置键名，与 `[fuzzy]` 分节的字段名一致；菜单按这个顺序列出。
    pub const NAMES: [&'static str; 9] = [
        "z_zh", "c_ch", "s_sh", "n_l", "f_h", "l_r", "an_ang", "en_eng", "in_ing",
    ];

    /// 按名字开一条规则（`z-zh` / `z_zh` / `zzh` 都认），CLI 参数用；不认识返回 `false`。
    pub fn enable(&mut self, name: &str) -> bool {
        self.set(name, true)
    }

    /// 按名字设置一条规则的开关；不认识返回 `false`。
    pub fn set(&mut self, name: &str, on: bool) -> bool {
        match self.flag_mut(name) {
            Some(flag) => {
                *flag = on;
                true
            }
            None => false,
        }
    }

    /// 按名字查一条规则是否开着；不认识返回 `false`。
    pub fn is_on(&self, name: &str) -> bool {
        let mut copy = *self;
        copy.flag_mut(name).is_some_and(|flag| *flag)
    }

    fn flag_mut(&mut self, name: &str) -> Option<&mut bool> {
        let normalized: String = name
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .collect::<String>()
            .to_ascii_lowercase();
        Some(match normalized.as_str() {
            "zzh" => &mut self.z_zh,
            "cch" => &mut self.c_ch,
            "ssh" => &mut self.s_sh,
            "nl" => &mut self.n_l,
            "fh" => &mut self.f_h,
            "lr" => &mut self.l_r,
            "anang" => &mut self.an_ang,
            "eneng" => &mut self.en_eng,
            "ining" => &mut self.in_ing,
            _ => return None,
        })
    }

    /// 把一串模式扩展成每个位置的多种写法（第一种是用户敲的，代价 0；模糊音写法扣 [`FUZZY_PENALTY`]）。
    pub fn expand(&self, patterns: &[SyllablePattern<'_>]) -> Expanded {
        Expanded::new(patterns.iter().map(|p| {
            let forms = self
                .alternatives(*p)
                .into_iter()
                .enumerate()
                .map(|(index, text)| (text, if index == 0 { 0.0 } else { FUZZY_PENALTY }))
                .collect();
            (forms, p.complete)
        }))
    }

    /// `syllable` 是不是 `typed` 按当前规则的一种模糊写法（不含它自己）。
    pub fn is_variant(&self, typed: &str, syllable: &str) -> bool {
        typed != syllable
            && self
                .alternatives(SyllablePattern::complete(typed))
                .iter()
                .any(|form| form == syllable)
    }

    /// 一个音节的全部写法，第一个是敲的原文；没开模糊音就只有它自己。
    /// 同一位置的写法互不覆盖（词库多写法查询的契约）：前缀 `zh` 换成 `z` 后只留 `z`。
    fn alternatives(&self, pattern: SyllablePattern<'_>) -> Vec<String> {
        let mut forms = vec![pattern.text.to_owned()];
        if !self.any() {
            return forms;
        }
        // 规则之间可以叠加（zhen → zeng），跑到不再增长为止；规则集很小，两轮就收敛
        let mut index = 0;
        while index < forms.len() {
            let current = forms[index].clone();
            for candidate in self.apply_once(&current, pattern.complete) {
                if !forms.contains(&candidate) {
                    forms.push(candidate);
                }
            }
            index += 1;
        }
        if !pattern.complete {
            // 前缀之间去覆盖：被别的更短前缀包含的去掉
            let snapshot = forms.clone();
            forms.retain(|f| !snapshot.iter().any(|o| o != f && f.starts_with(o.as_str())));
            // 原文若被去掉了，把覆盖它的那个放到第一位，保证第一种写法仍能匹配原文能匹配的
            if forms.first().map(String::as_str) != Some(pattern.text)
                && let Some(position) = forms
                    .iter()
                    .position(|f| pattern.text.starts_with(f.as_str()))
            {
                forms.swap(0, position);
            }
        }
        forms
    }

    /// 对一种写法各套一次规则得到的新写法（可能不合法，这里过滤掉）。
    fn apply_once(&self, form: &str, complete: bool) -> Vec<String> {
        let mut out = Vec::new();
        let mut swap_initial = |a: &str, b: &str| {
            if let Some(rest) = form.strip_prefix(a) {
                out.push(format!("{b}{rest}"));
            } else if let Some(rest) = form.strip_prefix(b) {
                out.push(format!("{a}{rest}"));
            }
        };
        // 先比双字母声母，`sh` 不能被当成 `s` 处理
        if self.z_zh {
            swap_initial("zh", "z");
        }
        if self.c_ch {
            swap_initial("ch", "c");
        }
        if self.s_sh {
            swap_initial("sh", "s");
        }
        if self.n_l {
            swap_initial("n", "l");
        }
        if self.f_h {
            swap_initial("f", "h");
        }
        if self.l_r {
            swap_initial("l", "r");
        }
        if complete {
            let mut swap_final = |a: &str, b: &str| {
                if let Some(head) = form.strip_suffix(a) {
                    out.push(format!("{head}{b}"));
                } else if let Some(head) = form.strip_suffix(b) {
                    out.push(format!("{head}{a}"));
                }
            };
            // 先比长的：`ang` 结尾的不能再被当成 `an`
            if self.an_ang {
                swap_final("ang", "an");
            }
            if self.en_eng {
                swap_final("eng", "en");
            }
            if self.in_ing {
                swap_final("ing", "in");
            }
        }
        out.retain(|f| {
            !f.is_empty()
                && f != form
                && if complete {
                    parser::is_syllable(f)
                } else {
                    parser::is_syllable(f) || parser::is_syllable_prefix(f)
                }
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn forms(rules: &FuzzyRules, pattern: SyllablePattern<'_>) -> Vec<String> {
        rules.alternatives(pattern)
    }

    #[test]
    fn initial_rules_apply_both_ways_and_combine_with_finals() {
        let rules = FuzzyRules::ALL;
        let mut zhen = forms(&rules, SyllablePattern::complete("zhen"));
        zhen.sort();
        assert_eq!(zhen, ["zen", "zeng", "zhen", "zheng"]);
        assert_eq!(forms(&rules, SyllablePattern::complete("lan"))[0], "lan");
        let mut lan = forms(&rules, SyllablePattern::complete("lan"));
        lan.sort();
        assert_eq!(lan, ["lan", "lang", "nan", "nang", "ran", "rang"]);
        // 不合法的写法不出：hua 没有 fua
        assert_eq!(forms(&rules, SyllablePattern::complete("hua")), ["hua"]);
    }

    #[test]
    fn only_enabled_rules_apply() {
        let mut rules = FuzzyRules::default();
        assert_eq!(forms(&rules, SyllablePattern::complete("zi")), ["zi"]);
        assert!(rules.enable("z-zh"));
        assert!(!rules.enable("q-x"));
        assert_eq!(
            forms(&rules, SyllablePattern::complete("zi")),
            ["zi", "zhi"]
        );
        assert_eq!(
            forms(&rules, SyllablePattern::complete("zhi")),
            ["zhi", "zi"]
        );
        // 韵母规则没开：fen 只有自己（f/h 也没开）
        assert_eq!(forms(&rules, SyllablePattern::complete("fen")), ["fen"]);
    }

    #[test]
    fn prefixes_only_get_initial_rules_and_never_overlap() {
        let rules = FuzzyRules::ALL;
        // 前缀 zh 换成 z 后 z 已覆盖 zh，只留 z
        assert_eq!(forms(&rules, SyllablePattern::prefix("zh")), ["z"]);
        assert_eq!(forms(&rules, SyllablePattern::prefix("z")), ["z"]);
        let mut fe = forms(&rules, SyllablePattern::prefix("fe"));
        fe.sort();
        assert_eq!(fe, ["fe", "he"]);
        // 前缀不套韵母规则
        assert_eq!(forms(&rules, SyllablePattern::prefix("xin")), ["xin"]);
        let expanded = rules.expand(&[
            SyllablePattern::complete("zi"),
            SyllablePattern::prefix("l"),
        ]);
        let positions = expanded.positions();
        assert_eq!(positions[0][0], SyllablePattern::complete("zi"));
        assert_eq!(positions[0][1], SyllablePattern::complete("zhi"));
        assert_eq!(positions[1].len(), 3); // l n r
        assert!(positions[1].iter().all(|p| !p.complete));
    }
}
