use qingjian_core::ShuangpinScheme;
use serde::{Deserialize, Serialize};

use super::{CandidateRenderer, LayoutMode, LogLevel, PreeditMode, ThemeMode};

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

    /// 候选窗口由青简渲染器还是系统原生绘制。
    pub renderer: CandidateRenderer,

    /// 候选窗口字体的字族名；空为系统字体。只对青简渲染器生效，没装这个字体时回到系统字体。
    pub font: String,

    /// 组句中的拼音显示在行内、候选窗口还是两处都显示。
    pub preedit: PreeditMode,

    /// 英文模式（Caps Lock 亮着）是否给英文候选（补全与拼错纠正）。关掉就是纯直通。
    pub english_candidates: bool,

    /// 中文模式下中英混输时中文候选总排在英文词前面。缺省关：拼音不像话的输入（`hello`）英文词排第一，
    /// 常在中文模式里打英文词的人不受影响；想要中文永远在前的自己打开。
    pub chinese_first: bool,

    /// 中文模式下不在组句时敲的标点转成全角（`，。？！` 等，数字后的 `.` 保持半角）。
    /// Windows 悬浮状态条上可点切换；macOS 在偏好设置中选择默认模式。
    pub full_width_punctuation: bool,

    /// 英文模式下的同一件事，中英各记一份；缺省半角。只有 Windows 用（macOS 英文模式一律半角）。
    pub english_full_width_punctuation: bool,

    /// 双拼方案：空串为全拼，否则 `xiaohe` / `ziranma` / `microsoft` / `sogou`（见 [`ShuangpinScheme`]）。
    pub shuangpin: String,

    /// 注音模式开关，大千键盘。
    pub zhuyin: bool,

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
            renderer: CandidateRenderer::default(),
            font: String::new(),
            preedit: PreeditMode::default(),
            english_candidates: true,
            chinese_first: false,
            full_width_punctuation: true,
            english_full_width_punctuation: false,
            shuangpin: String::new(),
            zhuyin: false,
            log_level: LogLevel::default(),
            input_log: true,
            learning: true,
            system_text_replacements: true,
        }
    }
}

impl GeneralConfig {
    /// 学习语言关着（`learning_language = "off"`）：候选旁不显示译文，生词标记与释义兜底也停。
    pub fn learning_language_off(&self) -> bool {
        self.learning_language
            .trim()
            .eq_ignore_ascii_case(LEARNING_LANGUAGE_OFF)
    }

    /// 双拼方案；没开或写得不认识时为 `None`（全拼）。
    pub fn shuangpin(&self) -> Option<ShuangpinScheme> {
        let key = self.shuangpin.trim();
        if key.is_empty() {
            return None;
        }
        match key.parse() {
            Ok(scheme) => Some(scheme),
            Err(_) => {
                tracing::warn!(key, "不认识的双拼方案，按全拼");
                None
            }
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
    fn shuangpin_is_off_by_default_and_unknown_names_fall_back() {
        let mut general = GeneralConfig::default();
        assert_eq!(general.shuangpin(), None);
        general.shuangpin = "xiaohe".to_owned();
        assert_eq!(general.shuangpin(), Some(ShuangpinScheme::Xiaohe));
        general.shuangpin = " Sogou ".to_owned();
        assert_eq!(general.shuangpin(), Some(ShuangpinScheme::Sogou));
        general.shuangpin = "flypy".to_owned();
        assert_eq!(general.shuangpin(), None);
    }
}
