use qingjian_core::ShuangpinScheme;
use serde::{Deserialize, Serialize};

use super::scheme::{Scheme, scheme_label};
use super::{CandidateRenderer, LayoutMode, LogLevel, PreeditMode, ShiftLetter, ThemeMode};

/// 每页最多几个候选：数字键只有 1–9。
pub const MAX_PAGE_SIZE: usize = 9;

/// 翻页键对的可选值，第一项是缺省：第一个键向前、第二个向后。
/// 缺省不用 `,` `.`：组句中敲逗号句号应该把首选上屏再补一个全角标点（`nihao,zaima` 一气打完），
/// 拿它们翻页就得先按空格再敲标点。选 `-` `=` 时组句中的 `-` 是翻页，不再进英文直输段（#43）。
pub const PAGE_KEY_OPTIONS: [&str; 3] = ["[]", ",.", "-="];

/// 缺省翻页键对，与 [`PAGE_KEY_OPTIONS`] 第一项一致。
pub const DEFAULT_PAGE_KEYS: (char, char) = ('[', ']');

/// `[general]` 分节：与具体功能无关的常规项。
/// `learning_language` 写这个值表示不显示译文。
pub const LEARNING_LANGUAGE_OFF: &str = "off";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// 学习语言（ISO 639-1，`en` / `ja` / `es`；`off` 不显示译文）：候选旁显示哪种语言的译文。要有对应的释义表文件才生效。
    pub learning_language: String,

    /// 每页候选数，1–9。
    pub page_size: usize,

    /// 翻页键对，两个字符：前一个上一页、后一个下一页。
    pub page_keys: String,

    /// 候选窗口外观。
    pub theme: ThemeMode,

    /// 候选窗口竖排 / 横排。
    pub layout: LayoutMode,

    /// 横排时上 / 下键把单行展开成多行矩阵（左 / 右键改为移动候选高亮，Esc 第一下先收回）。缺省关：横排下的按键与以前一样。只有 macOS 用。
    pub horizontal_grid: bool,

    /// 候选窗口由青简渲染器还是系统原生绘制。
    pub renderer: CandidateRenderer,

    /// 候选窗口字体的字族名；空为系统字体。只对青简渲染器生效，没装这个字体时回到系统字体。
    pub font: String,

    /// 组句中的拼音显示在行内、候选窗口还是两处都显示。
    pub preedit: PreeditMode,

    /// 英文模式（Caps Lock 亮着）是否给英文候选（补全与拼错纠正）。关掉就是纯直通。
    pub english_candidates: bool,

    /// 繁体输出模式。
    pub traditional: bool,
    /// 中文模式下中英混输时中文候选总排在英文词前面。缺省关：拼音不像话的输入（`hello`）英文词排第一，
    /// 常在中文模式里打英文词的人不受影响；想要中文永远在前的自己打开。
    pub chinese_first: bool,

    /// 中文模式下按住 Shift 敲的字母：交给应用（缺省）还是收进组句缓冲区参与匹配。
    /// 收进组句才能打出「C盘」这类混杂词（`Cpan` 与 `cpan` 一样匹配）。
    pub shift_letter: ShiftLetter,
    /// 内置英文模式：开着时单击切换键（`[shortcut] switch_mode`）或按 Caps Lock 能进英文模式。
    /// 关掉后青简保持中文模式，切换键与语言栏按钮都不再切过去；要打英文请用系统快捷键切到别的输入法。
    pub english_mode: bool,

    /// 中文模式下不在组句时敲的标点转成全角（`，。？！` 等，数字后的 `.` 保持半角）。
    /// Windows 悬浮状态条上可点切换；macOS 在偏好设置中选择默认模式。
    pub full_width_punctuation: bool,

    /// 英文模式下的同一件事，中英各记一份；缺省半角。只有 Windows 用（macOS 英文模式一律半角）。
    pub english_full_width_punctuation: bool,

    /// 辅码触发键：拼音打完之后敲它进辅码态，缺省 `;`。校验 = 单字符、ASCII 可打印、
    /// 非字母数字、非翻页键（见 [`qingjian_core::is_valid_aux_code_key`]）。
    pub aux_code_key: String,

    /// 候选上是否显示码（与译文拼成一条注记）。缺省关：竖排会挤、横排更难放下。
    pub aux_code_show: bool,

    /// 码段删空后是否留在辅码态：开（缺省）时 `;` 仍在、候选全部回来，空码段再按一次退格才退出；
    /// 关则删空即回纯拼音态。
    pub aux_code_keep_empty: bool,

    /// 拼音侧方案：`pinyin`（全拼，缺省）/ `xiaohe` / `ziranma` / `microsoft` / `sogou` / `abc` / `xiaolang` / `shoudao` / `zhuyin`
    /// / `none`（关，只用形码），见 [`Scheme`]。用不认识的写法时按全拼并警告。
    /// 缺省是空串：文件里没写这一项时要去看旧键，见 [`Self::scheme`]。
    pub scheme: String,

    /// 双拼方案下 preedit 显示原始按键字母（如 `ljse`）还是展开成全拼音节（`lan'se`）。
    /// 缺省关（展开成全拼音节）；常在双拼中打英文词或需要核对按键的人可打开。
    pub shuangpin_raw_preedit: bool,

    /// 形码侧方案：空串为关，`wubi86` 为五笔（86 版）。**与拼音同时开着就是混输**，见 [`Self::mixed`]。
    pub wubi: String,

    /// 旧键（2026-09-16 之前是 `[general] shuangpin`，空串为全拼）：只在 [`Self::scheme`] 里用来推断方案，
    /// 不再写出去；`scheme` 写了值就不看它。当时 `shuangpin` 与 `zhuyin` 是两个字段表达同一个维度。
    pub shuangpin: Option<String>,

    /// 旧键（同上的 `[general] zhuyin`）：同上。
    pub zhuyin: Option<bool>,

    /// 日志级别，缺省 info（不含用户敲的内容）。
    pub log_level: LogLevel,

    /// 输入日志：每次上屏记一行到数据目录的 `input-log.jsonl`（敲的键、看到的候选、选了什么），只写本机，
    /// 给离线回归评测与个人模型用。缺省开；关掉就不记，「高级」页可清空。
    pub input_log: bool,

    /// 学习输入习惯：按选择调整候选顺序、记新词与敲错纠正。关掉后不再记，已学的仍参与排序。
    pub learning: bool,

    /// 把系统的文本替换（macOS「键盘 → 文本替换」）并进自定义短语：输入码敲全后短语占该码最靠前的空位。只有 macOS 用。
    pub system_text_replacements: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            learning_language: "en".to_owned(),
            page_size: MAX_PAGE_SIZE,
            page_keys: PAGE_KEY_OPTIONS[0].to_owned(),
            theme: ThemeMode::default(),
            layout: LayoutMode::default(),
            horizontal_grid: false,
            renderer: CandidateRenderer::default(),
            font: String::new(),
            preedit: PreeditMode::default(),
            english_candidates: true,
            traditional: false,
            chinese_first: false,
            shift_letter: ShiftLetter::default(),
            english_mode: true,
            full_width_punctuation: true,
            english_full_width_punctuation: false,
            aux_code_key: qingjian_core::DEFAULT_AUX_CODE_KEY.to_string(),
            aux_code_show: false,
            aux_code_keep_empty: true,
            scheme: String::new(),
            shuangpin_raw_preedit: false,
            wubi: String::new(),
            shuangpin: None,
            zhuyin: None,
            log_level: LogLevel::default(),
            input_log: true,
            learning: true,
            system_text_replacements: true,
        }
    }
}

impl GeneralConfig {
    /// 拼音侧方案。`scheme` 没写时用旧键（`shuangpin` / `zhuyin`）推，都没有就是全拼。
    pub fn scheme(&self) -> Scheme {
        let key = self.scheme.trim();
        if !key.is_empty() {
            return match key.parse() {
                Ok(scheme) => scheme,
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
    pub fn shuangpin(&self) -> Option<ShuangpinScheme> {
        self.scheme().shuangpin()
    }

    /// 当前方案是不是大千注音。
    pub fn is_zhuyin(&self) -> bool {
        self.scheme() == Scheme::Zhuyin
    }

    /// 形码侧（五笔）开没开。`wubi` 认 `wubi86`；认不出来时警告并按关。
    /// 旧配置把五笔写在 `scheme` 里（`wubi86`），那时等价于「拼音关 + 五笔开」，[`Self::scheme`] 会解成
    /// [`Scheme::Off`]，这里跟着认下来，免得老配置升级后两个轴都关着、一个候选都不出。
    pub fn wubi(&self) -> bool {
        let key = self.wubi.trim();
        if key.is_empty() || key == "off" || key == "none" {
            return matches!(self.scheme.trim(), "wubi" | "wubi86");
        }
        if key == "wubi86" {
            return true;
        }
        tracing::warn!(key, "不认识的形码方案，按关");
        false
    }

    /// 拼音与形码同时开着 = 混输：两边都出候选，编码打全的形码词在前。
    pub fn mixed(&self) -> bool {
        self.scheme().is_on() && self.wubi()
    }

    /// 状态条上显示的输入方案名；见 [`scheme_label`]。
    pub fn scheme_label(&self) -> String {
        scheme_label(self.scheme(), self.wubi())
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
        general.scheme = "flypy".to_owned();
        assert_eq!(general.scheme(), Scheme::Pinyin);
    }

    #[test]
    fn the_two_axes_are_independent_and_their_combination_is_mixed_input() {
        let mut general = GeneralConfig::default();
        // 缺省：全拼，不开形码
        assert_eq!(general.scheme(), Scheme::Pinyin);
        assert!(!general.wubi());
        assert!(!general.mixed());

        // 双拼 alone
        general.scheme = "xiaohe".to_owned();
        assert!(!general.mixed());
        // 五笔 alone：拼音侧关掉
        general.scheme = "none".to_owned();
        general.wubi = "wubi86".to_owned();
        assert!(general.wubi());
        assert!(!general.mixed());
        // 组合：两边都开 = 混输
        general.scheme = "xiaohe".to_owned();
        assert!(general.mixed());
        // 形码写得不认识：按关，且不影响拼音侧
        general.wubi = "wubi98".to_owned();
        assert!(!general.wubi());
        assert_eq!(general.scheme(), Scheme::Shuangpin(ShuangpinScheme::Xiaohe));
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
        assert!(general.wubi());
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
