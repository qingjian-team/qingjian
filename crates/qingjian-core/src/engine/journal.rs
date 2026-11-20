//! Engine 的输入日志与统计。

use super::commit::{LastCommit, Transition};
use super::input_log::{MutedLogger, NoInputLogger};
use super::statistics::{NoUsageMeter, Typing, UsageMeter};

/// Engine 的输入日志与统计：写日志的 logger、日志序号、最近上屏与学习链、打字统计、上屏计量。
///
/// 这些都是「跟着输入走、跟着会话切换」的状态，单独放一块，Engine 的字段表里不再散着。
pub(crate) struct EngineJournal {
    /// 最近几次上屏各记了哪些学习、之后退格了几个字；用户把它们删掉重选时把学习退回去（见 `note_backspace`）。
    /// 最新的在末尾，最多留 `RECENT_COMMITS` 条。
    pub(crate) recent_commits: Vec<LastCommit>,

    /// 本次 commit 里记下的词转移，commit 结束时搬进 `last_commit`。
    pub(crate) recording: Vec<Transition>,

    /// 输入日志的落盘方；缺省不记，私密输入期间一律不记。
    pub(crate) logger: MutedLogger,

    /// 私密输入中（见 `set_private`）：不学、不记、不发云端。
    pub(crate) private: bool,

    /// 输入日志条目的序号。
    pub(crate) log_sequence: u64,

    /// 输入统计用的键数与打字时长。
    pub(crate) typing: Typing,

    /// 正在输入的应用标识，壳在焦点变化时给；写进输入日志。
    pub(crate) application: Option<String>,

    /// 上次记 `break` 之后有没有上屏过：没有就不再记，免得失焦一次记一条。
    pub(crate) committed_since_break: bool,

    /// 输入统计的累计方（打了多少字）；缺省不记。
    pub(crate) meter: Box<dyn UsageMeter>,
}

impl Default for EngineJournal {
    fn default() -> Self {
        Self {
            recent_commits: Vec::new(),
            recording: Vec::new(),
            logger: MutedLogger::new(Box::new(NoInputLogger)),
            private: false,
            log_sequence: 0,
            typing: Typing::default(),
            application: None,
            committed_since_break: false,
            meter: Box::new(NoUsageMeter),
        }
    }
}
