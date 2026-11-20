//! 输入模式与标点：中英模式开关、Shift / 小键盘行为、全半角与配对标点、标点上屏、组句保留。
//!
//! 这一组是 [`super::GeneralConfig`] 的扁平子集，序列化时由 `#[serde(flatten)]` 并回
//! `[general]` 分节，键名与旧扁平结构完全一致。

use serde::{Deserialize, Serialize};

use super::super::{NumpadDigit, ShiftLetter, ThirdInputMethod};

/// 输入模式与标点设置（`[general]` 下的输入行为相关键）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralInput {
    /// 中文模式下按住 Shift 敲的字母：交给应用（缺省）还是收进组句缓冲区参与匹配。
    /// 收进组句才能打出「C盘」这类混杂词（`Cpan` 与 `cpan` 一样匹配）。
    pub shift_letter: ShiftLetter,

    /// Windows 中文组句中小键盘数字选候选还是直接输入；主键盘数字不受影响。
    pub numpad_digit: NumpadDigit,

    /// 内置英文模式：开着时单击切换键（`[shortcut] switch_mode`）或按 Caps Lock 能进英文模式。
    /// 关掉后青简保持中文模式，切换键与语言栏按钮都不再切过去；要打英文请用系统快捷键切到别的输入法。
    pub english_mode: bool,

    /// 默认英文：Windows 是首次使用时，按应用记忆时用于没有记录的应用；Linux 是每个新输入上下文。
    pub default_english: bool,

    /// Windows：按应用 exe 名记住中英状态，仅保存在本次 Server 运行期间。
    pub remember_mode_per_app: bool,

    /// 连按两次 Shift 时切换到的第三输入法；缺省关。当前先保存用户选择，运行时接入随第三输入法实现补上。
    pub double_shift_input: ThirdInputMethod,

    /// 中文模式下不在组句时敲的标点转成全角（`，。？！` 等，数字后的 `.` 保持半角）。
    /// Windows 悬浮状态条上可点切换；macOS 在偏好设置中选择默认模式。
    pub full_width_punctuation: bool,

    /// 英文模式下的同一件事，中英各记一份；缺省半角。只有 Windows 用（macOS 英文模式一律半角）。
    pub english_full_width_punctuation: bool,

    /// 中文与日文模式下敲左括号、左引号、左书名号时连右半边一起补上，光标停在中间（#928）；缺省开。
    pub pair_punctuation: bool,

    /// 英文模式下的同一件事（半角括号与双引号）；缺省关：代码编辑器、浏览器地址栏自己会补，两边一起补会多出一个。
    pub english_pair_punctuation: bool,

    /// 中文标点开着也保持半角的符号，几个字符连写（如 `` ` `` 让它出反引号而不是 `·`，#893）；缺省空。
    /// 只认会转全角的标点，别的字符忽略。
    pub half_width_symbols: String,

    /// 组句中敲会转全角的标点（`,` `.` `?` `!` 等，翻页键除外）先把高亮候选上屏、再补该标点
    ///（`nihao,` 一气打完「你好，」）。缺省关：标点进英文直输段整段原样上屏，与以前一致。
    pub punct_commits: bool,

    /// 一段拼音分几次选完时，先选的词留在输入框里、整段选完才一起上屏，退格先撤回选择。缺省开；关掉是选了就上屏。
    pub hold_selections: bool,
}

impl Default for GeneralInput {
    fn default() -> Self {
        Self {
            shift_letter: ShiftLetter::default(),
            numpad_digit: NumpadDigit::default(),
            english_mode: true,
            default_english: false,
            remember_mode_per_app: false,
            double_shift_input: ThirdInputMethod::default(),
            full_width_punctuation: true,
            english_full_width_punctuation: false,
            pair_punctuation: true,
            english_pair_punctuation: false,
            half_width_symbols: String::new(),
            punct_commits: false,
            hold_selections: true,
        }
    }
}
