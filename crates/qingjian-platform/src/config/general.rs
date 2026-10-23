use qingjian_core::{ShuangpinScheme, TraditionalVariant};
use serde::{Deserialize, Serialize};

use super::scheme::Scheme;
use super::{
    Appearance, CandidateRenderer, EmojiCount, FontSize, KeyHintMode, LayoutMode,
    LegacyTraditional, LogLevel, PreeditMode, ShiftLetter,
};
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

/// `[general]` 分节：与具体功能无关的常规项。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// 学习语言（ISO 639-1，`en` / `ja` / `es` / `vi` / `de`；`off` 不显示译文）：候选旁显示哪种语言的译文。要有对应的释义表文件才生效。
    pub learning_language: String,

    /// 译词旁是否显示词性标注（v. / n. / adj. / phr. 等，`开发` → v. develop）。缺省开：关掉只留译词。
    /// 译文整体关掉（[`Self::learning_language`] 为 `off`）时此项无效果。
    pub translation_pos: bool,

    /// 候选窗口顶部的拼音行按高亮候选显示带调拼音（`gān'gà`），应用里的拼音不变。缺省关。
    pub show_tones: bool,

    /// 每页候选数，1–9。
    pub page_size: usize,

    /// 翻页键对，两个字符：前一个上一页、后一个下一页。
    pub page_keys: String,

    /// 候选窗口外观，读用 [`Self::appearance`]。文件里没写时为 `None`（字段级缺省），好认出旧写法：
    /// 2026-09-18 之前外观写在 `theme` 里（`theme = "dark"`）。
    #[serde(default)]
    pub appearance: Option<Appearance>,

    /// 候选窗口主题 id（主题目录名），读用 [`Self::theme_id`]。旧写法里这一项是外观，见上。
    pub theme: String,

    /// 候选窗口竖排 / 横排。
    pub layout: LayoutMode,

    /// 横排时上 / 下键把单行展开成多行矩阵（左 / 右键改为移动候选高亮，Esc 第一下先收回）。缺省关：横排下的按键与以前一样。
    pub horizontal_grid: bool,

    /// 候选窗口由青简渲染器还是系统原生绘制。
    pub renderer: CandidateRenderer,

    /// 候选窗口字体的字族名；空为系统字体。只对青简渲染器生效，没装这个字体时回到系统字体。
    pub font: String,

    /// 候选字字号（点），盖过主题的；0 用主题的。不限范围，不是正数的当 0。只对青简渲染器生效。
    pub candidate_font_size: FontSize,

    /// 译文字号（点），同上。
    pub annotation_font_size: FontSize,

    /// 候选窗口的过渡与循环动画。关掉与系统「减弱动态效果」一样：高亮直接跳过去，循环动画停在第一帧。
    pub animations: bool,

    /// 组句中的拼音显示在行内、候选窗口还是两处都显示。
    pub preedit: PreeditMode,

    /// 英文模式（Caps Lock 亮着）是否给英文候选（补全与拼错纠正）。关掉就是纯直通。
    pub english_candidates: bool,

    /// 中文模式下中英混输时中文候选总排在英文词前面。缺省关：拼音不像话的输入（`hello`）英文词排第一，
    /// 常在中文模式里打英文词的人不受影响；想要中文永远在前的自己打开。
    pub chinese_first: bool,

    /// 中文模式下整段是英文词或英文词的开头时给英文候选与补全（`hello`、`compa` → company）。关掉中文模式只出中文，英文模式照旧。
    pub english_in_chinese: bool,

    /// 候选后面一次最多配几个 emoji（`kaixin` → 开心 😄），0 时候选里只有字词。
    pub emoji: EmojiCount,

    /// 候选后面一次最多配几个颜文字（`ku` → 哭 (T_T)），缺省 1。
    pub kaomoji: EmojiCount,

    /// 候选后面一次最多配几个符号（`yuedengyu` → 约等于 ≈），缺省 1。
    pub symbols: EmojiCount,

    /// 中西文混排自动空格：汉字与半角字母 / 数字之间补一个空格（「用 Go 写了 30 行」）。缺省关；目前只有 macOS 用。
    pub auto_space: bool,

    /// 中文模式下按住 Shift 敲的字母：交给应用（缺省）还是收进组句缓冲区参与匹配。
    /// 收进组句才能打出「C盘」这类混杂词（`Cpan` 与 `cpan` 一样匹配）。
    pub shift_letter: ShiftLetter,
    /// 内置英文模式：开着时单击切换键（`[shortcut] switch_mode`）或按 Caps Lock 能进英文模式。
    /// 关掉后青简保持中文模式，切换键与语言栏按钮都不再切过去；要打英文请用系统快捷键切到别的输入法。
    pub english_mode: bool,

    /// 默认英文：Windows 是首次使用时，按应用记忆时用于没有记录的应用；Linux 是每个新输入上下文。
    pub default_english: bool,

    /// Windows：按应用 exe 名记住中英状态，仅保存在本次 Server 运行期间。
    pub remember_mode_per_app: bool,

    /// 中文模式下不在组句时敲的标点转成全角（`，。？！` 等，数字后的 `.` 保持半角）。
    /// Windows 悬浮状态条上可点切换；macOS 在偏好设置中选择默认模式。
    pub full_width_punctuation: bool,

    /// 英文模式下的同一件事，中英各记一份；缺省半角。只有 Windows 用（macOS 英文模式一律半角）。
    pub english_full_width_punctuation: bool,

    /// 组句中敲会转全角的标点（`,` `.` `?` `!` 等，翻页键除外）先把高亮候选上屏、再补该标点
    ///（`nihao,` 一气打完「你好，」）。缺省关：标点进英文直输段整段原样上屏，与以前一致。
    pub punct_commits: bool,

    /// 辅码触发键：拼音打完之后敲它进辅码态，缺省 `;`。校验 = 单字符、ASCII 可打印、
    /// 非字母数字、非翻页键（见 [`qingjian_core::is_valid_aux_code_key`]）。
    pub aux_code_key: String,

    /// 候选上是否显示码（与译文拼成一条注记）。缺省关：竖排会挤、横排更难放下。
    pub aux_code_show: bool,

    /// 码段删空后是否留在辅码态：开（缺省）时 `;` 仍在、候选全部回来，空码段再按一次退格才退出；
    /// 关则删空即回纯拼音态。
    pub aux_code_keep_empty: bool,

    /// 拼音侧方案：`pinyin`（全拼，缺省）/ `xiaohe` / `ziranma` / `microsoft` / `sogou` / `abc` / `xiaolang` / `shoudao`
    /// / `jiajia` / `ziguang` / `zhuyin`
    /// / `none`（关，只用形码），见 [`Scheme`]。用不认识的写法时按全拼并警告。
    /// 缺省是空串：文件里没写这一项时要去看旧键，见 [`Self::scheme`]。
    pub scheme: String,

    /// 双拼方案下 preedit 显示原始按键字母（如 `ljse`）还是展开成全拼音节（`lan'se`）。
    /// 缺省关（展开成全拼音节）；常在双拼中打英文词或需要核对按键的人可打开。
    pub shuangpin_raw_preedit: bool,

    /// 一段拼音分几次选完时，先选的词留在输入框里、整段选完才一起上屏，退格先撤回选择。缺省开；关掉是选了就上屏。
    pub hold_selections: bool,

    /// 双拼键位提示：在候选窗旁画出当前方案的键位，敲了声母后标出能接的韵母键。见 [`KeyHintMode`]。
    pub key_hint: KeyHintMode,

    /// 键位提示窗口的不透明度（百分比，30–100），见 [`Self::key_hint_opacity`]。
    pub key_hint_opacity: u8,

    /// 键位提示键盘的大小（百分比，60–150，在候选字号的比例之上再缩放），见 [`Self::key_hint_size`]。
    pub key_hint_size: u8,

    /// 形码侧方案：`off` 为关，`wubi86` 为内置五笔，其他是用户目录 `schemes/<id>/` 下的方案（见 [`crate::code_schemes`]）。
    /// **与拼音同时开着就是混输**，见 [`Self::mixed`]。缺省是空串：文件里没写这一项时要去看旧键，见 [`Self::code`]。
    pub code: String,

    /// 旧键（2026-10-20 之前的 `[general] wubi`，空串为关、`wubi86` 为开）：只在 [`Self::code`] 里用，不再写出去。
    pub wubi: Option<String>,

    /// 旧键（2026-09-16 之前是 `[general] shuangpin`，空串为全拼）：只在 [`Self::scheme`] 里用来推断方案，
    /// 不再写出去；`scheme` 写了值就不看它。当时 `shuangpin` 与 `zhuyin` 是两个字段表达同一个维度。
    pub shuangpin: Option<String>,

    /// 旧键（同上的 `[general] zhuyin`）：同上。
    pub zhuyin: Option<bool>,

    /// 繁体输出：`off` / `taiwan` / `hongkong` / `standard`，空串为没写，读法见 [`Self::traditional`]。
    /// 词库与学习数据始终是简体，只在出 Core 时换（见 [`TraditionalVariant`]）。
    /// 用字符串不用枚举：配置里出现这一版不认识的写法（退回旧版本时）只让这一项回缺省，不让整个配置解析失败。
    /// 缺省值是 `off`，但配置里没写这一项时读成空串（字段级 `default`），才轮得到旧键。
    #[serde(default)]
    pub traditional_variant: String,

    /// 旧键 `[general] traditional`（0.2.1 及以前是布尔，繁体三档的开发版写过字符串）：只在 [`Self::traditional`] 里用，
    /// `traditional_variant` 写了值就不看它。新写法换了键名，旧版本读到新配置时才不会因类型不符整个解析失败。
    pub traditional: Option<LegacyTraditional>,

    /// 日志级别，缺省 info（不含用户敲的内容）。
    pub log_level: LogLevel,

    /// 输入日志：每次上屏记一行到数据目录的 `input-log.jsonl`（敲的键、看到的候选、选了什么），只写本机，
    /// 给离线回归评测与个人模型用。缺省开；关掉就不记，「高级」页可清空。
    pub input_log: bool,

    /// 学习输入习惯：按选择调整候选顺序、记新词与敲错纠正。关掉后不再记，已学的仍参与排序。
    pub learning: bool,

    /// 把系统的文本替换（macOS「键盘 → 文本替换」）并进自定义短语：输入码敲全后短语排在候选前面，同权重时排在用户自己的短语之后。只有 macOS 用。
    pub system_text_replacements: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            learning_language: "en".to_owned(),
            translation_pos: true,
            show_tones: false,
            page_size: MAX_PAGE_SIZE,
            page_keys: PAGE_KEY_OPTIONS[0].to_owned(),
            appearance: Some(Appearance::default()),
            theme: DEFAULT_THEME.to_owned(),
            layout: LayoutMode::default(),
            horizontal_grid: false,
            renderer: CandidateRenderer::default(),
            font: String::new(),
            candidate_font_size: FontSize::default(),
            annotation_font_size: FontSize::default(),
            animations: true,
            preedit: PreeditMode::default(),
            english_candidates: true,
            chinese_first: false,
            english_in_chinese: true,
            emoji: EmojiCount::default(),
            kaomoji: EmojiCount::new(1),
            symbols: EmojiCount::new(1),
            auto_space: false,
            shift_letter: ShiftLetter::default(),
            english_mode: true,
            default_english: false,
            remember_mode_per_app: false,
            full_width_punctuation: true,
            english_full_width_punctuation: false,
            punct_commits: false,
            aux_code_key: qingjian_core::DEFAULT_AUX_CODE_KEY.to_string(),
            aux_code_show: false,
            aux_code_keep_empty: true,
            scheme: String::new(),
            shuangpin_raw_preedit: false,
            hold_selections: true,
            key_hint: KeyHintMode::default(),
            key_hint_opacity: DEFAULT_KEY_HINT_OPACITY,
            key_hint_size: 100,
            code: String::new(),
            wubi: None,
            shuangpin: None,
            zhuyin: None,
            traditional_variant: TraditionalVariant::Off.key().to_owned(),
            traditional: None,
            log_level: LogLevel::default(),
            input_log: true,
            learning: true,
            system_text_replacements: true,
        }
    }
}

impl GeneralConfig {
    /// 繁体输出：`traditional_variant` 优先；没写时看旧键，`true` 是台湾字形（与 0.2.1 的输出相同），字符串按同一套写法读。
    pub fn traditional(&self) -> TraditionalVariant {
        let key = self.traditional_variant.trim();
        if !key.is_empty() {
            return traditional_from_key(key);
        }
        match &self.traditional {
            Some(LegacyTraditional::Flag(true)) => TraditionalVariant::Taiwan,
            Some(LegacyTraditional::Key(key)) => traditional_from_key(key.trim()),
            Some(LegacyTraditional::Flag(false)) | None => TraditionalVariant::Off,
        }
    }

    /// 候选窗口外观。`appearance` 没写时看旧写法（`theme` 写的是 system / light / dark），都没有就跟随系统。
    pub fn appearance(&self) -> Appearance {
        self.appearance
            .or_else(|| Appearance::from_key(self.theme.trim()))
            .unwrap_or_default()
    }

    /// 候选窗口主题 id。没写、空串或旧写法（外观词）都是内置主题。
    pub fn theme_id(&self) -> &str {
        let id = self.theme.trim();
        if id.is_empty() || Appearance::from_key(id).is_some() {
            DEFAULT_THEME
        } else {
            id
        }
    }

    /// 拼音侧方案。`scheme` 没写时用旧键（`shuangpin` / `zhuyin`）推，都没有就是全拼。
    pub fn scheme(&self) -> Scheme {
        let key = self.scheme.trim();
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
        if self.zhuyin == Some(true) {
            tracing::info!("[general] zhuyin 已并入 scheme，可改成 scheme = \"zhuyin\"");
            return Scheme::Zhuyin;
        }
        let Some(legacy) = self
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
        self.learning_language
            .trim()
            .eq_ignore_ascii_case(LEARNING_LANGUAGE_OFF)
    }

    /// 当前方案是双拼时是哪一套；不是双拼时为 `None`。
    /// 键位提示窗口的不透明度（0–1），写出范围的收到 30%–100%：太透明就看不见了。
    pub fn key_hint_opacity(&self) -> f32 {
        f32::from(self.key_hint_opacity.clamp(30, 100)) / 100.0
    }

    /// 键位提示键盘的缩放倍数，写出范围的收到 60%–150%。
    pub fn key_hint_size(&self) -> f32 {
        f32::from(self.key_hint_size.clamp(60, 150)) / 100.0
    }

    pub fn shuangpin(&self) -> Option<ShuangpinScheme> {
        self.scheme().shuangpin()
    }

    /// 拼音侧选的用户双拼方案（id）；不是 [`Scheme::Custom`] 时为 `None`。这里只读配置，方案在不在由壳去找。
    pub fn custom_shuangpin(&self) -> Option<String> {
        (self.scheme() == Scheme::Custom).then(|| self.scheme.trim().to_owned())
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
        let key = match self.code.trim() {
            "" => match self.wubi.as_deref().map(str::trim) {
                Some(key) if !key.is_empty() => key,
                _ if matches!(self.scheme.trim(), "wubi" | "wubi86") => WUBI86,
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
        let mut chars = self.aux_code_key.chars();
        match (chars.next(), chars.next()) {
            (Some(key), None) if qingjian_core::is_valid_aux_code_key(key, self.page_keys()) => key,
            _ => qingjian_core::DEFAULT_AUX_CODE_KEY,
        }
    }

    /// 夹到合法范围的每页候选数。
    pub fn page_size(&self) -> usize {
        self.page_size.clamp(1, MAX_PAGE_SIZE)
    }

    /// 翻页键对；写得不对（不是两个不同的 ASCII 可见字符）时退回缺省。
    pub fn page_keys(&self) -> (char, char) {
        let mut chars = self.page_keys.chars();
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
mod tests {
    use super::*;

    #[test]
    fn horizontal_grid_is_off_unless_switched_on() {
        assert!(!GeneralConfig::default().horizontal_grid);
        let general: GeneralConfig = toml::from_str("layout = \"horizontal\"\n").unwrap();
        assert!(!general.horizontal_grid);
        let general: GeneralConfig = toml::from_str("horizontal_grid = true\n").unwrap();
        assert!(general.horizontal_grid);
    }

    #[test]
    fn page_size_and_keys_are_sanitized() {
        let mut general = GeneralConfig::default();
        assert_eq!(general.page_size(), 9);
        assert_eq!(general.page_keys(), ('[', ']'));
        general.page_size = 0;
        general.page_keys = ",.".to_owned();
        assert_eq!(general.page_size(), 1);
        assert_eq!(general.page_keys(), (',', '.'));
        general.page_size = 42;
        general.page_keys = "ab".to_owned();
        assert_eq!(general.page_size(), 9);
        assert_eq!(general.page_keys(), ('[', ']'));
        general.page_keys = ",,".to_owned();
        assert_eq!(general.page_keys(), ('[', ']'));
    }

    /// 组句中标点先上屏候选缺省关（`[general] punct_commits`），保持标点进英文直输段的老行为。
    #[test]
    fn punct_commits_defaults_off() {
        assert!(!GeneralConfig::default().punct_commits);
        let general: GeneralConfig = toml::from_str("punct_commits = true\n").unwrap();
        assert!(general.punct_commits);
    }

    #[test]
    fn aux_code_key_falls_back_to_the_default() {
        let mut general = GeneralConfig::default();
        assert_eq!(general.aux_code_key(), ';');
        assert!(!general.aux_code_show);
        general.aux_code_key = "/".to_owned();
        assert_eq!(general.aux_code_key(), '/');
        for bad in ["", "ab", "a", "1", "[", "中"] {
            general.aux_code_key = bad.to_owned();
            assert_eq!(general.aux_code_key(), ';', "{bad}");
        }
    }

    /// 码删空保持辅码态的开关缺省开（`[general] aux_code_keep_empty`）。
    #[test]
    fn aux_code_keep_empty_defaults_on() {
        assert!(GeneralConfig::default().aux_code_keep_empty);
    }

    /// 词性标注缺省开（旧配置文件没这一项时行为不变），显式写 false 才关。
    #[test]
    fn translation_pos_defaults_on() {
        assert!(GeneralConfig::default().translation_pos);
        let general: GeneralConfig = toml::from_str("").unwrap();
        assert!(general.translation_pos);
        let general: GeneralConfig = toml::from_str("translation_pos = false\n").unwrap();
        assert!(!general.translation_pos);
    }

    #[test]
    fn scheme_defaults_to_pinyin_and_unknown_names_fall_back() {
        let mut general = GeneralConfig::default();
        assert_eq!(general.scheme(), Scheme::Pinyin);
        general.scheme = "xiaohe".to_owned();
        assert_eq!(general.shuangpin(), Some(ShuangpinScheme::Xiaohe));
        general.scheme = " Sogou ".to_owned();
        assert_eq!(general.shuangpin(), Some(ShuangpinScheme::Sogou));
        general.scheme = "xiaolang".to_owned();
        assert_eq!(general.shuangpin(), Some(ShuangpinScheme::Xiaolang));
        general.scheme = "none".to_owned();
        assert_eq!(general.scheme(), Scheme::Off);
        assert!(!general.scheme().is_on());
        assert_eq!(general.custom_shuangpin(), None);
        general.scheme = " my-sp ".to_owned();
        assert_eq!(general.scheme(), Scheme::Custom);
        assert_eq!(general.custom_shuangpin().as_deref(), Some("my-sp"));
        assert_eq!(general.shuangpin(), None);
        general.scheme = "Flypy!".to_owned();
        assert_eq!(general.scheme(), Scheme::Pinyin);
        assert_eq!(general.custom_shuangpin(), None);
    }

    #[test]
    fn the_two_axes_are_independent_and_their_combination_is_mixed_input() {
        let mut general = GeneralConfig::default();
        // 缺省：全拼，不开形码
        assert_eq!(general.scheme(), Scheme::Pinyin);
        assert_eq!(general.code(), None);
        assert!(!general.mixed());

        // 双拼 alone
        general.scheme = "xiaohe".to_owned();
        assert!(!general.mixed());
        // 形码 alone：拼音侧关掉
        general.scheme = "none".to_owned();
        general.code = "huma".to_owned();
        assert_eq!(general.code().as_deref(), Some("huma"));
        assert!(!general.mixed());
        // 组合：两边都开 = 混输
        general.scheme = "xiaohe".to_owned();
        assert!(general.mixed());
        // 关形码不影响拼音侧
        general.code = "off".to_owned();
        assert_eq!(general.code(), None);
        assert_eq!(general.scheme(), Scheme::Shuangpin(ShuangpinScheme::Xiaohe));
    }

    #[test]
    fn files_written_before_the_code_key_keep_their_wubi() {
        let parse = |text: &str| toml::from_str::<GeneralConfig>(text).unwrap().code();
        assert_eq!(parse("wubi = \"wubi86\"\n").as_deref(), Some("wubi86"));
        assert_eq!(parse("wubi = \"\"\n"), None);
        assert_eq!(parse(""), None);
        // 新键写了就以它为准，`off` 关掉也不回头看旧键
        assert_eq!(parse("code = \"off\"\nwubi = \"wubi86\"\n"), None);
        assert_eq!(
            parse("code = \"huma\"\nwubi = \"wubi86\"\n").as_deref(),
            Some("huma")
        );
    }

    #[test]
    fn the_old_wubi_scheme_spelling_still_turns_wubi_on() {
        // 2026-09-16 之前五笔是 `scheme = "wubi86"`（单选）。升级后两个轴都得跟上，
        // 否则老配置会变成「拼音关 + 形码关」，一个字都打不出来。
        let general = GeneralConfig {
            scheme: "wubi86".to_owned(),
            ..GeneralConfig::default()
        };
        assert_eq!(general.scheme(), Scheme::Off);
        assert_eq!(general.code().as_deref(), Some("wubi86"));
        assert!(!general.mixed());
    }

    #[test]
    fn files_written_before_the_scheme_key_keep_their_scheme() {
        let parse = |text: &str| toml::from_str::<GeneralConfig>(text).unwrap().scheme();
        assert_eq!(
            parse("shuangpin = \"xiaohe\"\n"),
            Scheme::Shuangpin(ShuangpinScheme::Xiaohe)
        );
        assert_eq!(parse("zhuyin = true\n"), Scheme::Zhuyin);
        assert_eq!(parse("shuangpin = \"\"\nzhuyin = false\n"), Scheme::Pinyin);
        assert_eq!(parse(""), Scheme::Pinyin);
        // 新键写了就以它为准
        assert_eq!(
            parse("scheme = \"pinyin\"\nshuangpin = \"xiaohe\"\n"),
            Scheme::Pinyin
        );
    }

    /// 0.2.1 及以前的 `traditional = true` 照旧出台湾字形；开发版在旧键里写的字符串也认；新键优先；不认识的写法只让这一项回简体。
    #[test]
    fn traditional_variant_wins_over_the_legacy_key() {
        let parse = |text: &str| toml::from_str::<GeneralConfig>(text).unwrap().traditional();
        assert_eq!(parse("traditional = true"), TraditionalVariant::Taiwan);
        assert_eq!(parse("traditional = false"), TraditionalVariant::Off);
        assert_eq!(parse(""), TraditionalVariant::Off);
        assert_eq!(
            parse("traditional = true\ntraditional_variant = \"off\""),
            TraditionalVariant::Off
        );
        assert_eq!(
            parse("traditional_variant = \"hongkong\""),
            TraditionalVariant::Hongkong
        );
        assert_eq!(
            parse("traditional_variant = \"macau\"\npage_size = 5"),
            TraditionalVariant::Off
        );
        assert_eq!(parse("traditional = \"off\""), TraditionalVariant::Off);
        assert_eq!(
            parse("traditional = \"hongkong\""),
            TraditionalVariant::Hongkong
        );
        assert_eq!(
            parse("traditional = \"hongkong\"\ntraditional_variant = \"standard\""),
            TraditionalVariant::Standard
        );
        let unknown = toml::from_str::<GeneralConfig>("traditional = \"macau\"\npage_size = 5")
            .expect("旧键写错不能让整个配置解析失败");
        assert_eq!(unknown.traditional(), TraditionalVariant::Off);
        assert_eq!(unknown.page_size, 5);
    }

    #[test]
    fn legacy_shuangpin_and_zhuyin_keys_are_read_but_scheme_wins() {
        let mut general = GeneralConfig {
            scheme: String::new(),
            ..GeneralConfig::default()
        };
        assert_eq!(general.scheme(), Scheme::Pinyin);
        general.shuangpin = Some("xiaohe".to_owned());
        assert_eq!(general.scheme(), Scheme::Shuangpin(ShuangpinScheme::Xiaohe));
        general.zhuyin = Some(true);
        assert_eq!(general.scheme(), Scheme::Zhuyin);
        assert!(general.is_zhuyin());
        // 旧配置里两个都写是不合法的，新键写了就不看它们
        general.scheme = "pinyin".to_owned();
        assert_eq!(general.scheme(), Scheme::Pinyin);
        assert!(!general.is_zhuyin());
    }
}
