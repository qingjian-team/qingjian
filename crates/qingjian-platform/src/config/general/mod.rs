//! `[general]` 分节：与具体功能无关的常规项。
//!
//! 64 个配置键按域归到五个扁平子结构（[`GeneralDisplay`] / [`GeneralCandidates`] /
//! [`GeneralInput`] / [`GeneralSchemes`] / [`GeneralAdvanced`]），序列化时由
//! `#[serde(flatten)]` 并回 `[general]` 分节：TOML / JSON 键路径、默认值与反序列化容错
//! 与拆分前的扁平结构完全一致，分组只影响 Rust 侧的字段归属。

mod advanced;
mod candidates;
mod display;
mod input;
mod schemes;

pub use advanced::GeneralAdvanced;
pub use candidates::GeneralCandidates;
pub use display::GeneralDisplay;
pub use input::GeneralInput;
pub use schemes::GeneralSchemes;

use qingjian_core::{ShuangpinScheme, TraditionalVariant};
use serde::{Deserialize, Serialize};

use super::scheme::Scheme;
use super::{Appearance, LegacyTraditional};
use crate::code_schemes::{self, WUBI86};

/// 每页最多几个候选：数字键只有 1–9。
pub const MAX_PAGE_SIZE: usize = 9;

/// 翻页键对的可选值，第一项是缺省：第一个键向前、第二个向后。
/// 缺省不用 `,` `.`：组句中敲逗号句号应该把首选上屏再补一个全角标点（`nihao,zaima` 一气打完），
/// 拿它们翻页就得先按空格再敲标点。选 `-` `=` 时组句中的 `-` 是翻页，不再进英文直输段（#43）。
pub const PAGE_KEY_OPTIONS: [&str; 3] = ["[]", ",.", "-="];

/// 内置主题的 id。`system` / `light` / `dark` 是外观的写法，不能当主题 id（旧配置里 `theme` 写的是它们）。
pub const DEFAULT_THEME: &str = "qingjian";

/// 缺省翻页键对，与 [`PAGE_KEY_OPTIONS`] 第一项一致。
pub const DEFAULT_PAGE_KEYS: (char, char) = ('[', ']');

/// 键位提示窗口缺省的不透明度（百分比）。
pub const DEFAULT_KEY_HINT_OPACITY: u8 = 85;

pub use qingjian_core::LEARNING_LANGUAGE_OFF;

/// `[general]` 分节：与具体功能无关的常规项。字段按域收进下面五个扁平子结构，见模块文档。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// 外观、布局、字体、动画与键位提示。
    #[serde(flatten)]
    pub display: GeneralDisplay,

    /// 译文、候选内容、英文候选与 emoji / 颜文字 / 符号配给。
    #[serde(flatten)]
    pub candidates: GeneralCandidates,

    /// 中英模式、标点与组句保留。
    #[serde(flatten)]
    pub input: GeneralInput,

    /// 拼音 / 形码方案、双拼显示与辅码。
    #[serde(flatten)]
    pub schemes: GeneralSchemes,

    /// 繁体字形、界面语言与学习调试。
    #[serde(flatten)]
    pub advanced: GeneralAdvanced,
}

/// 界面语言跟随系统的配置写法。
pub const UI_LANGUAGE_SYSTEM: &str = "system";

/// 配置里没写界面语言时用的语言。
pub const UI_LANGUAGE_DEFAULT: &str = "zh-Hans";

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            display: GeneralDisplay::default(),
            candidates: GeneralCandidates::default(),
            input: GeneralInput::default(),
            schemes: GeneralSchemes::default(),
            advanced: GeneralAdvanced::default(),
        }
    }
}

impl GeneralConfig {
    /// 交给 `bamboo_slip_i18n::set_locale` 的界面语言：没写时简体中文（用户多是系统设成英文的中文用户），`system` 跟随系统（空串）。
    pub fn ui_locale(&self) -> &str {
        match self.advanced.ui_language.trim() {
            "" => UI_LANGUAGE_DEFAULT,
            UI_LANGUAGE_SYSTEM => "",
            tag => tag,
        }
    }

    /// 繁体输出：`traditional_variant` 优先；没写时看旧键，`true` 是台湾字形（与 0.2.1 的输出相同），字符串按同一套写法读。
    pub fn traditional(&self) -> TraditionalVariant {
        let key = self.advanced.traditional_variant.trim();
        if !key.is_empty() {
            return traditional_from_key(key);
        }
        match &self.advanced.traditional {
            Some(LegacyTraditional::Flag(true)) => TraditionalVariant::Taiwan,
            Some(LegacyTraditional::Key(key)) => traditional_from_key(key.trim()),
            Some(LegacyTraditional::Flag(false)) | None => TraditionalVariant::Off,
        }
    }

    /// 简繁快捷键从简体切过去的那一档繁体：没写、写的是 `off` 或不认识时用台湾字形（与 0.2.1 的开关相同）。
    pub fn last_traditional(&self) -> TraditionalVariant {
        let key = self.advanced.last_traditional.trim();
        if key.is_empty() {
            return TraditionalVariant::Taiwan;
        }
        match traditional_from_key(key) {
            TraditionalVariant::Off => TraditionalVariant::Taiwan,
            variant => variant,
        }
    }

    /// 候选窗口外观。`appearance` 没写时看旧写法（`theme` 写的是 system / light / dark），都没有就跟随系统。
    pub fn appearance(&self) -> Appearance {
        self.display
            .appearance
            .or_else(|| Appearance::from_key(self.display.theme.trim()))
            .unwrap_or_default()
    }

    /// 候选窗口主题 id。没写、空串或旧写法（外观词）都是内置主题。
    pub fn theme_id(&self) -> &str {
        let id = self.display.theme.trim();
        if id.is_empty() || Appearance::from_key(id).is_some() {
            DEFAULT_THEME
        } else {
            id
        }
    }

    /// 拼音侧方案。`scheme` 没写时用旧键（`shuangpin` / `zhuyin`）推，都没有就是全拼。
    pub fn scheme(&self) -> Scheme {
        let schemes = &self.schemes;
        let key = schemes.scheme.trim();
        if !key.is_empty() {
            return match key.parse() {
                Ok(scheme) => scheme,
                Err(_) if code_schemes::is_valid_id(key) => Scheme::Custom,
                Err(_) => {
                    tracing::warn!(key, "不认识的拼音方案，按全拼");
                    Scheme::Pinyin
                }
            };
        }
        if schemes.zhuyin == Some(true) {
            tracing::info!("[general] zhuyin 已并入 scheme，可改成 scheme = \"zhuyin\"");
            return Scheme::Zhuyin;
        }
        let Some(legacy) = schemes
            .shuangpin
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            return Scheme::Pinyin;
        };
        match legacy.parse::<ShuangpinScheme>() {
            Ok(scheme) => {
                tracing::info!(
                    key = scheme.key(),
                    "[general] shuangpin 已并入 scheme，可改成它"
                );
                Scheme::Shuangpin(scheme)
            }
            Err(_) => {
                tracing::warn!(key = legacy, "不认识的双拼方案，按全拼");
                Scheme::Pinyin
            }
        }
    }

    /// 学习语言关着（`learning_language = "off"`）：候选旁不显示译文，生词标记与释义兜底也停。
    pub fn learning_language_off(&self) -> bool {
        self.candidates.learning_language_off()
    }

    /// 键位提示窗口的不透明度（0–1），写出范围的收到 30%–100%：太透明就看不见了。
    pub fn key_hint_opacity(&self) -> f32 {
        f32::from(self.display.key_hint_opacity.clamp(30, 100)) / 100.0
    }

    /// 键位提示键盘的缩放倍数，写出范围的收到 60%–150%。
    pub fn key_hint_size(&self) -> f32 {
        f32::from(self.display.key_hint_size.clamp(60, 150)) / 100.0
    }

    pub fn shuangpin(&self) -> Option<ShuangpinScheme> {
        self.scheme().shuangpin()
    }

    /// 拼音侧选的用户双拼方案（id）；不是 [`Scheme::Custom`] 时为 `None`。这里只读配置，方案在不在由壳去找。
    pub fn custom_shuangpin(&self) -> Option<String> {
        (self.scheme() == Scheme::Custom).then(|| self.schemes.scheme.trim().to_owned())
    }

    /// 当前方案是不是大千注音。
    pub fn is_zhuyin(&self) -> bool {
        self.scheme() == Scheme::Zhuyin
    }

    /// 形码侧选了哪套方案（id），关着为 `None`。这里只读配置，id 对应的方案在不在由壳去找。
    ///
    /// `code` 没写时看旧键：`wubi = "wubi86"` 是开；更早的配置把五笔写在 `scheme` 里（`wubi86`），
    /// 那时等价于「拼音关 + 五笔开」，[`Self::scheme`] 会解成 [`Scheme::Off`]，这里跟着认下来，
    /// 免得老配置升级后两个轴都关着、一个候选都不出。
    pub fn code(&self) -> Option<String> {
        let schemes = &self.schemes;
        let key = match schemes.code.trim() {
            "" => match schemes.wubi.as_deref().map(str::trim) {
                Some(key) if !key.is_empty() => key,
                _ if matches!(schemes.scheme.trim(), "wubi" | "wubi86") => WUBI86,
                _ => return None,
            },
            key => key,
        };
        (!matches!(key, "off" | "none")).then(|| key.to_owned())
    }

    /// 拼音与形码同时开着 = 混输：两边都出候选，编码打全的形码词在前。
    pub fn mixed(&self) -> bool {
        self.scheme().is_on() && self.code().is_some()
    }

    /// 辅码触发键；写得不对（不是单个合法字符，或撞上当前翻页键 / 拼音隔音符 `'`）时退回缺省 `;`。
    pub fn aux_code_key(&self) -> char {
        let mut chars = self.schemes.aux_code_key.chars();
        match (chars.next(), chars.next()) {
            (Some(key), None)
                if qingjian_core::is_valid_aux_code_key(key, self.page_keys()) =>
            {
                key
            }
            _ => qingjian_core::DEFAULT_AUX_CODE_KEY,
        }
    }

    /// 设置里的候选字字重：100–900 取最近的整百，0 和范围外的为 `None`（用主题的）。
    pub fn candidate_font_weight(&self) -> Option<u16> {
        (100..=900)
            .contains(&self.display.candidate_font_weight)
            .then(|| {
                ((self.display.candidate_font_weight + 50) / 100 * 100).min(900)
            })
    }

    /// 夹到合法范围的每页候选数。
    pub fn page_size(&self) -> usize {
        self.candidates.page_size.clamp(1, MAX_PAGE_SIZE)
    }

    /// 夹到 1–[`qingjian_core::ENGLISH_CANDIDATES_MAX`] 的英文候选数；写 0 或更大都收到边界。
    pub fn english_max_in_chinese(&self) -> usize {
        self.candidates
            .english_max_in_chinese
            .clamp(1, qingjian_core::ENGLISH_CANDIDATES_MAX)
    }

    /// 夹到 1–9 的英文最高位次；1 即不限制。
    pub fn english_min_position(&self) -> usize {
        self.candidates.english_min_position.clamp(1, MAX_PAGE_SIZE)
    }

    /// 翻页键对；写得不对（不是两个不同的 ASCII 可见字符）时退回缺省。
    pub fn page_keys(&self) -> (char, char) {
        let mut chars = self.candidates.page_keys.chars();
        match (chars.next(), chars.next(), chars.next()) {
            (Some(previous), Some(next), None)
                if previous != next
                    && previous.is_ascii_graphic()
                    && next.is_ascii_graphic()
                    && !previous.is_ascii_alphanumeric()
                    && !next.is_ascii_alphanumeric() =>
            {
                (previous, next)
            }
            _ => DEFAULT_PAGE_KEYS,
        }
    }
}

/// 繁体输出的字符串写法；不认识的只让这一项回简体。
fn traditional_from_key(key: &str) -> TraditionalVariant {
    TraditionalVariant::ALL
        .into_iter()
        .find(|variant| variant.key() == key)
        .unwrap_or_else(|| {
            tracing::warn!(key, "不认识的繁体输出写法，按简体");
            TraditionalVariant::Off
        })
}

#[cfg(test)]
mod characterization;
#[cfg(test)]
mod tests;
