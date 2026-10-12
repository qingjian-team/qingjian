//! 可挂起的会话输入状态；词库与用户学习服务仍由同一个 Engine 持有。

use super::commit::CommitChain;
use super::{Engine, LastCommit, Transition};
use crate::{Composition, InputHistory, Language, Punctuation};
use std::time::Instant;

/// 一个输入上下文的组句、标点配对和学习链；不复制词库或落盘服务。
#[derive(Default)]
pub struct EngineSession {
    /// 拼音与光标位置。
    composition: Composition,

    /// 本段输入的译词抽样轮次，恢复组句时保持所见译词。
    translation_round: u64,

    /// 当前中英文模式。
    english_mode: bool,

    /// 中文标点与引号配对。
    punctuation: Punctuation,

    /// 最近上屏记录，供撤销错误学习。
    recent_commits: Vec<LastCommit>,

    /// 本次上屏记录的词转移。
    recording: Vec<Transition>,

    /// 首次退格前的输入，供重敲记录。
    retype_snapshot: Option<String>,

    /// 等待写入日志的直通字符。
    passthrough_pending: String,

    /// 本轮候选翻页次数。
    page_turns: u32,

    /// 本轮组句开始时刻。
    composition_started: Option<Instant>,

    /// 当前输入应用标识。
    application: Option<String>,

    /// 上次学习链断开后是否已有上屏。
    committed_since_break: bool,

    /// 当前可见候选的译词。
    displayed: Vec<(Language, String)>,

    /// 本会话最近的上屏前文。
    history: InputHistory,

    /// 本会话连续上屏的学习链。
    chain: CommitChain,
}

impl Engine {
    /// 切换保存的输入上下文。调用方还应恢复该上下文的私密状态。
    /// 异步结果与查询缓存不跨上下文复用，用户词频和词库仍是进程内唯一实例。
    pub fn swap_session(&mut self, session: &mut EngineSession) {
        self.cancel_prediction();
        self.set_rescoring_context(None);
        std::mem::swap(&mut self.composition, &mut session.composition);
        std::mem::swap(&mut self.translation_round, &mut session.translation_round);
        std::mem::swap(&mut self.english_mode, &mut session.english_mode);
        std::mem::swap(&mut self.punctuation, &mut session.punctuation);
        std::mem::swap(&mut self.recent_commits, &mut session.recent_commits);
        std::mem::swap(&mut self.recording, &mut session.recording);
        std::mem::swap(&mut self.retype_snapshot, &mut session.retype_snapshot);
        std::mem::swap(
            &mut self.passthrough_pending,
            &mut session.passthrough_pending,
        );
        std::mem::swap(&mut self.page_turns, &mut session.page_turns);
        std::mem::swap(
            &mut self.composition_started,
            &mut session.composition_started,
        );
        std::mem::swap(&mut self.application, &mut session.application);
        std::mem::swap(
            &mut self.committed_since_break,
            &mut session.committed_since_break,
        );
        std::mem::swap(&mut self.displayed, &mut session.displayed);
        std::mem::swap(&mut self.history, &mut session.history);
        std::mem::swap(&mut self.chain, &mut session.chain);
        self.forget_span_cache();
        *self.correction_cache.borrow_mut() = None;
        *self.last_query.borrow_mut() = None;
    }
}

impl EngineSession {
    /// 丢弃本上下文的输入状态，不写日志、不学习，也不把私密文本带到下一次普通输入。
    pub fn discard_input(&mut self) {
        self.composition.clear();
        self.english_mode = false;
        self.punctuation = Punctuation::default();
        self.recent_commits.clear();
        self.recording.clear();
        self.retype_snapshot = None;
        self.passthrough_pending.clear();
        self.page_turns = 0;
        self.composition_started = None;
        self.committed_since_break = false;
        self.displayed.clear();
        self.history.clear();
        self.chain = CommitChain::default();
    }
}

impl Engine {
    /// 在隐私边界丢弃当前装入的输入状态。与 [`Self::set_private`] 分开，避免平台壳
    /// 在组句第一帧后报告隐私状态时意外清掉新输入。
    pub fn discard_input(&mut self) {
        self.cancel_prediction();
        self.composition.clear();
        self.english_mode = false;
        self.punctuation = Punctuation::default();
        self.recent_commits.clear();
        self.recording.clear();
        self.retype_snapshot = None;
        self.passthrough_pending.clear();
        self.page_turns = 0;
        self.composition_started = None;
        self.committed_since_break = false;
        self.displayed.clear();
        self.history.clear();
        self.chain = CommitChain::default();
        self.rescoring_before = None;
        self.last_prediction_scope.clear();
        self.last_question_guess.clear();
        *self.neural_cache.borrow_mut() = super::rescoring::NeuralCache::default();
        *self.correction_cache.borrow_mut() = None;
        *self.last_query.borrow_mut() = None;
        self.forget_span_cache();
        self.traditional_map.borrow_mut().clear();
    }
}
