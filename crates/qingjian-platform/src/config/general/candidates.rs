//! 译文与候选内容：学习语言、词性 / 音调标注、每页候选、英文候选与混排策略、emoji / 颜文字 / 符号配给、空格。
//!
//! 这一组是 [`super::GeneralConfig`] 的扁平子集，序列化时由 `#[serde(flatten)]` 并回
//! `[general]` 分节，键名与旧扁平结构完全一致。

use serde::{Deserialize, Serialize};

use super::super::{EmojiCount, PreeditMode};
use super::{LEARNING_LANGUAGE_OFF, MAX_PAGE_SIZE, PAGE_KEY_OPTIONS};

/// 译文与候选内容设置（`[general]` 下的候选相关键）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralCandidates {
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

    /// 组句中的拼音显示在行内、候选窗口还是两处都显示。
    pub preedit: PreeditMode,

    /// 英文模式（Caps Lock 亮着）是否给英文候选（补全与拼错纠正）。关掉就是纯直通。
    pub english_candidates: bool,

    /// 中文模式下中英混输时中文候选总排在英文词前面。缺省关：拼音不像话的输入（`hello`）英文词排第一，
    /// 常在中文模式里打英文词的人不受影响；想要中文永远在前的自己打开。
    pub chinese_first: bool,

    /// 拼写纠错：拼音敲错时（切不动、多敲少敲、敲到旁边的键）给纠正后的候选。缺省开；
    /// 敲的拼音照原样读得通时纠正只当备选，不排第一。关掉后只按敲的原样（与模糊音）读，学过的敲错也不再用。
    pub spelling_correction: bool,

    /// 中文模式下整段是英文词或英文词的开头时给英文候选与补全（`hello`、`compa` → company）。关掉中文模式只出中文，英文模式照旧。
    pub english_in_chinese: bool,

    /// 中文模式下英文候选（精确词 + 补全）最多几个。缺省 4（1 条精确词 + 3 条补全），1 时只留最靠前的一条。
    pub english_max_in_chinese: usize,

    /// 有中文候选时英文最高排到第几位。缺省 1（不限制）；没有中文候选时英文照旧第一。
    pub english_min_position: usize,

    /// 英文候选旁显示音标（`cargo /ˈkɑrɡoʊ/`）。缺省关，关掉时英文候选与以前完全一样。
    /// 只影响英文候选：中文候选的译词、注音都不走这条路。查不到音标的词不出错、也不显示。
    pub show_english_pronunciation: bool,

    /// 候选后面一次最多配几个 emoji（`kaixin` → 开心 😄），0 时候选里只有字词。
    pub emoji: EmojiCount,

    /// 候选后面一次最多配几个颜文字（`ku` → 哭 (T_T)），缺省 1。
    pub kaomoji: EmojiCount,

    /// 候选后面一次最多配几个符号（`yuedengyu` → 约等于 ≈），缺省 1。
    pub symbols: EmojiCount,

    /// 中西文混排自动空格：汉字与半角字母 / 数字之间补一个空格（「用 Go 写了 30 行」）。缺省关；目前只有 macOS 用。
    pub auto_space: bool,
}

impl Default for GeneralCandidates {
    fn default() -> Self {
        Self {
            learning_language: "en".to_owned(),
            translation_pos: true,
            show_tones: false,
            page_size: MAX_PAGE_SIZE,
            page_keys: PAGE_KEY_OPTIONS[0].to_owned(),
            preedit: PreeditMode::default(),
            english_candidates: true,
            chinese_first: false,
            spelling_correction: true,
            english_in_chinese: true,
            english_max_in_chinese: qingjian_core::ENGLISH_CANDIDATES_MAX,
            english_min_position: 1,
            show_english_pronunciation: false,
            emoji: EmojiCount::default(),
            kaomoji: EmojiCount::new(1),
            symbols: EmojiCount::new(1),
            auto_space: false,
        }
    }
}

impl GeneralCandidates {
    /// 学习语言关着（`learning_language = "off"`）：候选旁不显示译文，生词标记与释义兜底也停。
    pub fn learning_language_off(&self) -> bool {
        self.learning_language
            .trim()
            .eq_ignore_ascii_case(LEARNING_LANGUAGE_OFF)
    }
}
