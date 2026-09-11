use serde::{Deserialize, Serialize};

use super::InputSource;

/// 一次上屏。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitEntry {
    /// 本进程内单调递增的序号，撤销条目用它指回来。
    pub id: u64,

    /// 查询时整段作用域的原始键（回放评测就是把它重新喂给引擎）；双拼下是双拼键，纠错时是敲错的原串。
    #[serde(default)]
    pub scope: String,

    /// 这次上屏消耗掉的那部分原始键（`scope` 的前缀）。
    pub keys: String,

    /// 查询时整段作用域的切分（`'` 连接；纠错生效时是纠正后的拼音）。
    pub pinyin: String,

    /// 拼写纠错是否生效。
    pub corrected: bool,

    /// 上屏的文字。
    pub text: String,

    /// 来源。
    pub source: InputSource,

    /// 选的是查询时候选列表里的第几个（从 0 数）；不在列表里（晚到的云端词、原样上屏）为 `None`。
    pub index: Option<usize>,

    /// 查询时排在前面的几个候选文本，用来离线算首选命中率。
    pub top: Vec<String>,

    /// 双拼方案的键（`xiaohe`），全拼为空。
    pub scheme: String,

    /// 是否在英文模式。
    pub english: bool,

    /// 查询时的候选顺序经过了神经重排（本地整句模型的分已经进了排序）。
    #[serde(default)]
    pub rescored: bool,

    /// 正在输入的应用（macOS bundle identifier / Windows exe 名）；壳没给为 `None`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,

    /// 这段组句里翻了几页候选（翻回来也算一次）。
    #[serde(default)]
    pub pages: u32,

    /// 这段组句从第一键到这次上屏的毫秒数；一段拼音分几次上屏时每次都从第一键算。
    #[serde(default)]
    pub ms: u64,
}

/// 日志里的一条。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum InputLogEntry {
    /// 上屏。
    Commit(CommitEntry),

    /// 用户把上一次上屏的词整个退格删掉又对同一段拼音选了别的：那次选错了。
    Retract {
        /// 被撤销的那条上屏的 `id`。
        of: u64,

        /// 被撤销的文字。
        text: String,

        /// 改选的文字。
        chosen: String,
    },

    /// 输入法启动或日志打开时记一次：之后的条目是哪个版本、什么配置下记的。
    Session {
        /// 日志格式版本（见 `docs/plan/model-eval.md`）。
        v: u32,

        /// 输入法版本号。
        version: String,

        /// 平台（`macos` / `windows` / `cli`）。
        platform: String,

        /// 本地整句模型（神经重打分）开没开。
        model: bool,

        /// 双拼方案的键，全拼为空。
        scheme: String,
    },

    /// 用户重打了键：组句里退格之后上屏的键串与第一次退格前不同，或上屏后删掉重打了相近但不同的键。
    /// 敲错纠正的真实样本。
    Retype {
        /// 退格前的键串。
        before: String,

        /// 最终上屏的键串。
        after: String,

        /// 对应的上屏 `id`（组句内是这次上屏，跨上屏是被删掉的那次）。
        of: u64,
    },

    /// 组句之外直接交给应用的字符（标点、回车、空格、英文模式的字母），连续的攒成一条：文本流的分隔与段落边界。
    Passthrough {
        /// 攒下的字符。
        text: String,
    },

    /// 云端联想的结果到达并展示了：给过用户什么，紧接着的 `commit` 说明接没接受。
    Prediction {
        /// 请求时的作用域（原始键）。
        scope: String,

        /// 云端词文本。
        words: Vec<String>,

        /// 整句补全。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sentence: Option<String>,
    },

    /// 上文链断了（切换应用、失焦、清空）：文本流的段落边界。
    Break {
        /// 断开之后所在的应用；壳没给为 `None`。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        app: Option<String>,
    },
}

/// 当前日志格式版本。
pub const INPUT_LOG_VERSION: u32 = 1;
