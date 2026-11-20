//! 繁体字形、界面语言与学习调试：简繁档位、界面语言、日志级别、输入日志、学习开关、系统短语并入。
//!
//! 这一组是 [`super::GeneralConfig`] 的扁平子集，序列化时由 `#[serde(flatten)]` 并回
//! `[general]` 分节，键名与旧扁平结构完全一致。

use serde::{Deserialize, Serialize};

use super::super::{LegacyTraditional, LogLevel};
use super::TraditionalVariant;

/// 繁体、界面语言与学习调试设置（`[general]` 下的高级 / 调试相关键）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralAdvanced {
    /// 繁体输出：`off` / `taiwan` / `hongkong` / `standard`，空串为没写，读法见 [`super::GeneralConfig::traditional`]。
    /// 词库与学习数据始终是简体，只在出 Core 时换（见 [`TraditionalVariant`]）。
    /// 用字符串不用枚举：配置里出现这一版不认识的写法（退回旧版本时）只让这一项回缺省，不让整个配置解析失败。
    /// 缺省值是 `off`，但配置里没写这一项时读成空串（字段级 `default`），才轮得到旧键。
    #[serde(default)]
    pub traditional_variant: String,

    /// 旧键 `[general] traditional`（0.2.1 及以前是布尔，繁体三档的开发版写过字符串）：只在 [`super::GeneralConfig::traditional`] 里用，
    /// `traditional_variant` 写了值就不看它。新写法换了键名，旧版本读到新配置时才不会因类型不符整个解析失败。
    pub traditional: Option<LegacyTraditional>,

    /// 简繁快捷键切回繁体时用的字形，快捷键切到简体时记下切走前那一档；读法见 [`super::GeneralConfig::last_traditional`]。
    pub last_traditional: String,

    /// 设置程序等界面用的语言（`zh-Hans`、`en`、`vi`…），`system` 跟随系统，空串是缺省的简体中文；读用 [`super::GeneralConfig::ui_locale`]。
    /// 只管界面文字，不影响打字与译文。
    pub ui_language: String,

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

impl Default for GeneralAdvanced {
    fn default() -> Self {
        Self {
            traditional_variant: TraditionalVariant::Off.key().to_owned(),
            traditional: None,
            last_traditional: String::new(),
            ui_language: String::new(),
            log_level: LogLevel::default(),
            input_log: true,
            learning: true,
            system_text_replacements: true,
        }
    }
}
