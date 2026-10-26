//! 拼音侧方案：全拼 / 双拼九套 / 大千注音 / 关。配置项 `[general] scheme` 的值。
//!
//! 「输入方案是配置项，不是模式」：中英切换始终是布尔，换方案不改变别的方案的既定按键行为。
//!
//! **拼音与形码是两条独立的轴**：这里只管拼音侧（读法），形码侧看 `[general] code`（见 [`crate::code_schemes`]）。
//! 两边都开就是混输，所以旧版那个「单选的 `scheme`」表达不了，2026-09-16 拆开——
//! 当时五笔是 `scheme = "wubi86"`，等价于「拼音关 + 五笔开」。

use std::fmt;
use std::str::FromStr;

use qingjian_core::ShuangpinScheme;

/// 拼音侧方案。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Scheme {
    /// 全拼。
    #[default]
    Pinyin,

    /// 双拼，九套键位见 [`ShuangpinScheme`]。
    Shuangpin(ShuangpinScheme),

    /// 用户自己的双拼方案：`[general] scheme` 写的是 `schemes/<id>/` 的 id（见 [`crate::shuangpin_schemes`]），
    /// 具体是哪套由 [`super::GeneralConfig::custom_shuangpin`] 给。不在 [`Self::ALL`] 里：有哪些要去目录里找。
    Custom,

    /// 大千注音。
    Zhuyin,

    /// 关：拼音侧不参与查询，只用形码。形码也关着的话没有候选。
    Off,
}

impl Scheme {
    /// 全部方案，设置界面与状态条按这个顺序列。
    pub const ALL: [Self; 12] = [
        Self::Pinyin,
        Self::Shuangpin(ShuangpinScheme::Xiaohe),
        Self::Shuangpin(ShuangpinScheme::Ziranma),
        Self::Shuangpin(ShuangpinScheme::Microsoft),
        Self::Shuangpin(ShuangpinScheme::Sogou),
        Self::Shuangpin(ShuangpinScheme::Abc),
        Self::Shuangpin(ShuangpinScheme::Xiaolang),
        Self::Shuangpin(ShuangpinScheme::Shoudao),
        Self::Shuangpin(ShuangpinScheme::Jiajia),
        Self::Shuangpin(ShuangpinScheme::Ziguang),
        Self::Zhuyin,
        Self::Off,
    ];

    /// 配置文件里的写法。`const`：设置界面按 [`Self::ALL`] 直接建常量表，不再手抄一份。
    pub const fn key(self) -> &'static str {
        match self {
            Self::Pinyin => "pinyin",
            Self::Shuangpin(scheme) => scheme.key(),
            Self::Custom => "custom",
            Self::Zhuyin => "zhuyin",
            Self::Off => "none",
        }
    }

    /// 界面上的名字。`const` 的理由同上。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pinyin => "全拼",
            Self::Shuangpin(scheme) => scheme.label(),
            Self::Custom => "自定义双拼",
            Self::Zhuyin => "大千注音",
            Self::Off => "关（只用形码）",
        }
    }

    /// 这套方案是双拼时是哪一套；装配引擎用（不是双拼时为 `None`）。
    pub const fn shuangpin(self) -> Option<ShuangpinScheme> {
        match self {
            Self::Shuangpin(scheme) => Some(scheme),
            _ => None,
        }
    }

    /// 这套方案是否为双拼方案（内置或用户的）。
    pub const fn is_shuangpin(self) -> bool {
        matches!(self, Self::Shuangpin(_) | Self::Custom)
    }

    /// 拼音侧参不参与查询。
    pub const fn is_on(self) -> bool {
        !matches!(self, Self::Off)
    }

    /// 日志里的写法：全拼为空串（老日志里没有这个字段就是全拼），其余同 [`Self::key`]。
    pub const fn log_key(self) -> &'static str {
        match self {
            Self::Pinyin => "",
            other => other.key(),
        }
    }
}

/// 状态条上显示的方案名，形码在前（与候选顺序一致）；全拼且不开形码时为空串。
/// 全拼只在同时开着形码时才写出来——单开全拼是缺省，标它没意义。
/// `shuangpin` 是装上的用户双拼方案的名字（[`Scheme::Custom`] 没装上时按全拼算，与引擎一致），
/// `code` 是形码方案的名字（[`crate::code_schemes::CodeSchemeEntry::label`]）。
///
/// 做成跟 [`Scheme`] 与装上的方案一起算的自由函数，而不是存进 `RouterConfig`：
/// 存下来的话它会与那几项冗余、手搓配置的地方就会漂移（测试正是这么发现的）。
pub fn scheme_label(pinyin: Scheme, shuangpin: Option<&str>, code: Option<&str>) -> String {
    let (pinyin, pinyin_label) = pinyin_side(pinyin, shuangpin);
    let mut parts: Vec<&str> = Vec::new();
    if let Some(code) = code {
        parts.push(code);
    }
    if pinyin.is_on() && (code.is_some() || pinyin != Scheme::Pinyin) {
        parts.push(pinyin_label);
    }
    parts.join(" + ")
}

/// 主题显示的方案名：同 [`scheme_label`]，只是单开全拼时也写出「全拼」（主题要一个总有内容的名字）。
pub fn scheme_name(pinyin: Scheme, shuangpin: Option<&str>, code: Option<&str>) -> String {
    let label = scheme_label(pinyin, shuangpin, code);
    if label.is_empty() {
        pinyin_side(pinyin, shuangpin).1.to_owned()
    } else {
        label
    }
}

/// 拼音侧实际生效的方案与它的名字：用户双拼没装上就是全拼。
fn pinyin_side(pinyin: Scheme, shuangpin: Option<&str>) -> (Scheme, &str) {
    match (pinyin, shuangpin) {
        (Scheme::Custom, Some(label)) => (pinyin, label),
        (Scheme::Custom, None) => (Scheme::Pinyin, Scheme::Pinyin.label()),
        (other, _) => (other, other.label()),
    }
}

impl FromStr for Scheme {
    type Err = String;

    /// 认不出来的写法报错，由调用方决定退回什么（配置层退回全拼并警告）。
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text.trim() {
            "" | "pinyin" => Ok(Self::Pinyin),
            "zhuyin" => Ok(Self::Zhuyin),
            // 旧配置把五笔写在这一栏（2026-09-16 之前），等价于「拼音关」
            "wubi" | "wubi86" => Ok(Self::Off),
            "none" | "off" => Ok(Self::Off),
            other => other.parse().map(Self::Shuangpin),
        }
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

    #[test]
    fn scheme_name_always_says_something() {
        assert_eq!(scheme_name(Scheme::Pinyin, None, None), "全拼");
        assert_eq!(scheme_name(Scheme::Zhuyin, None, None), "大千注音");
        assert_eq!(
            scheme_name(Scheme::Pinyin, None, Some("虎码")),
            "虎码 + 全拼"
        );
        assert_eq!(scheme_name(Scheme::Off, None, Some("虎码")), "虎码");
        assert_eq!(
            scheme_name(Scheme::Custom, Some("我的双拼"), None),
            "我的双拼"
        );
        assert_eq!(
            scheme_label(Scheme::Custom, Some("我的双拼"), None),
            "我的双拼"
        );
        assert_eq!(scheme_name(Scheme::Custom, None, None), "全拼");
        assert_eq!(scheme_label(Scheme::Custom, None, None), "");
    }

    #[test]
    fn parses_every_key_it_prints() {
        for scheme in Scheme::ALL {
            assert_eq!(scheme.key().parse::<Scheme>(), Ok(scheme), "{scheme}");
        }
    }

    #[test]
    fn empty_and_unknown_spellings() {
        assert_eq!("".parse::<Scheme>(), Ok(Scheme::Pinyin));
        assert_eq!(" pinyin ".parse::<Scheme>(), Ok(Scheme::Pinyin));
        assert_eq!("none".parse::<Scheme>(), Ok(Scheme::Off));
        assert_eq!("off".parse::<Scheme>(), Ok(Scheme::Off));
        assert!("flypy".parse::<Scheme>().is_err());
    }

    #[test]
    fn the_old_wubi_spelling_means_turn_the_pinyin_side_off() {
        // 2026-09-16 之前五笔是 `scheme = "wubi86"`，那时它是一个单选的方案
        assert_eq!("wubi86".parse::<Scheme>(), Ok(Scheme::Off));
        assert_eq!("wubi".parse::<Scheme>(), Ok(Scheme::Off));
        assert!(!Scheme::Off.is_on());
        assert!(Scheme::Pinyin.is_on());
    }
}
