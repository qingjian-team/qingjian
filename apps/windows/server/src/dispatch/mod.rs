//! 协议分派：把 DLL 发来的 [`ClientMessage`] 交给 Engine，产出回给 DLL 的 [`ServerMessage`]。

mod composed;
mod config;
mod keys;

use std::collections::HashSet;
use std::time::{Duration, Instant};

use qingjian_core::{Candidate, CandidateLayout, CandidateList, CloudWord, Engine};
use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyOutcome, PreeditKind, PreeditSegment, ServerMessage,
    SessionId,
};

use self::composed::Composed;
pub use self::config::RouterConfig;

/// 学习数据落盘的间隔（与 macOS 壳一致）。Server 没有定时器，借每条消息的节拍看时间。
const LEARNING_FLUSH_INTERVAL: Duration = Duration::from_secs(60);

/// 一次按键对组句的影响。
enum Effect {
    /// 缓冲变了（敲字 / 退格 / 移光标 / 上屏），要重建 [`Composed`]；带本次要上屏的文本。
    Changed(Option<String>),

    /// 只挪了高亮 / 翻页，不重建。
    Navigated,

    /// 不吃，交还应用。
    Passthrough,
}

/// 同一时刻只有一个应用有键盘焦点，所以像 macOS 那样用一个 Engine 持当前组句；
/// 焦点切到别的会话时先清掉上一个会话的残留。云端候选 / 整句补全由 DLL 组句期间定时 [`ClientMessage::Poll`] 拉取。
pub struct Router {
    /// 输入内核，进程内唯一。
    engine: Engine,

    /// 每页候选数 / 云端槽位 / 排布 / 外观 / 翻页键。
    config: RouterConfig,

    /// 活跃会话。
    sessions: HashSet<SessionId>,

    /// 当前持有组句的会话。
    focused: Option<SessionId>,

    /// 当前组句的展示状态；没在组句时为 `None`。
    composed: Option<Composed>,

    /// 整句补全：画在 preedit 右侧、Tab 上屏。缓冲变化时清空，异步到达时填。
    sentence: Option<String>,

    /// 当前高亮候选在候选布局里的下标（跨页）。缓冲变化时归 0。
    highlight: usize,

    /// 上次把学习数据落盘的时间。
    last_flush: Instant,
}

impl Router {
    pub fn new(engine: Engine, config: RouterConfig) -> Self {
        Self {
            engine,
            config: RouterConfig {
                page_size: config.page_size.max(1),
                ..config
            },
            sessions: HashSet::new(),
            focused: None,
            composed: None,
            sentence: None,
            highlight: 0,
            last_flush: Instant::now(),
        }
    }

    /// 处理一条消息；`None` 表示不用回话。到点就把学习数据落盘。
    pub fn handle(&mut self, message: ClientMessage) -> Option<ServerMessage> {
        let response = self.dispatch(message);
        if self.last_flush.elapsed() >= LEARNING_FLUSH_INTERVAL {
            self.flush_learning();
        }
        response
    }

    /// 把学习数据（词频 / 用户词 / 统计 / 词汇记录 / 个人释义表）落盘；没有新数据时是空操作。
    pub fn flush_learning(&mut self) {
        self.engine.flush_learning();
        self.last_flush = Instant::now();
    }

    fn dispatch(&mut self, message: ClientMessage) -> Option<ServerMessage> {
        match message {
            ClientMessage::OpenSession { session } => {
                self.sessions.insert(session);
                tracing::debug!(?session, "会话打开");
                None
            }
            ClientMessage::Key { session, event } => Some(self.handle_key(session, event)),
            ClientMessage::Poll { session } => Some(self.handle_poll(session)),
            ClientMessage::Commit { session } => {
                // TODO(windows)：焦点离开时把 preedit 上屏；协议还没有「无按键的上屏」响应，先丢弃。
                self.reset_composition();
                tracing::debug!(?session, "结束组句");
                None
            }
            ClientMessage::Surrounding {
                session,
                request,
                text,
            } => {
                // TODO(windows)：交给 Engine 做整句前文（set_rescoring_context）。
                tracing::trace!(
                    ?session,
                    request,
                    chars = text.chars().count(),
                    "收到上下文"
                );
                None
            }
            ClientMessage::CloseSession { session } => {
                self.sessions.remove(&session);
                if self.focused == Some(session) {
                    self.reset_composition();
                    self.focused = None;
                }
                // 应用退出 / 切走输入法都会关会话：趁机落盘，Server 被杀也最多丢这之后的。
                self.flush_learning();
                tracing::debug!(?session, "会话关闭");
                None
            }
        }
    }

    /// 当前活跃会话数。
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// 清掉当前组句、展示状态与在飞的云联想请求。
    fn reset_composition(&mut self) {
        self.engine.break_chain();
        self.engine.clear();
        self.cancel_prediction();
        self.composed = None;
        self.sentence = None;
        self.highlight = 0;
    }

    fn cancel_prediction(&mut self) {
        if self.engine.prediction_enabled() {
            self.engine.cancel_prediction();
        }
    }

    /// 焦点切到别的会话：清掉上一个会话残留的组句。
    fn ensure_focus(&mut self, session: SessionId) {
        if self.focused != Some(session) {
            self.reset_composition();
            self.focused = Some(session);
        }
    }

    fn handle_key(&mut self, session: SessionId, event: KeyEvent) -> ServerMessage {
        self.ensure_focus(session);
        let (commit, outcome) = match self.apply_key(&event) {
            Effect::Changed(commit) => {
                self.recompose();
                (commit, KeyOutcome::Consumed)
            }
            Effect::Navigated => (None, KeyOutcome::Consumed),
            Effect::Passthrough => (None, KeyOutcome::Passthrough),
        };
        // 顺手收一次已到的云联想结果，连续打字时不必等 Poll 定时器。
        self.poll_prediction();
        ServerMessage::KeyResult {
            session,
            outcome,
            commit,
            frame: self.current_frame(),
        }
    }

    /// 把一个按键作用到 Engine / 高亮上。字母总是进组句；其余键只在组句中才处理。
    fn apply_key(&mut self, event: &KeyEvent) -> Effect {
        if let Some(letter) = event.character.filter(char::is_ascii_alphabetic) {
            self.engine.push(letter.to_ascii_lowercase());
            return Effect::Changed(None);
        }
        if self.engine.composition().is_empty() {
            return Effect::Passthrough;
        }
        match event.virtual_key {
            keys::BACK => {
                self.engine.backspace();
                Effect::Changed(None)
            }
            keys::ESCAPE => {
                self.engine.clear();
                Effect::Changed(None)
            }
            keys::RETURN => Effect::Changed(Some(self.engine.take_raw())),
            keys::TAB => match self.sentence.take() {
                Some(sentence) => Effect::Changed(Some(self.engine.accept_prediction(&sentence))),
                // 没有整句补全：Tab 交还应用（缩进 / 跳焦点）。
                None => Effect::Passthrough,
            },
            keys::SPACE => Effect::Changed(self.commit_index(self.highlight)),
            keys::DOWN => {
                self.move_highlight(1);
                Effect::Navigated
            }
            keys::UP => {
                self.move_highlight(-1);
                Effect::Navigated
            }
            keys::NEXT => {
                self.page(1);
                Effect::Navigated
            }
            keys::PRIOR => {
                self.page(-1);
                Effect::Navigated
            }
            keys::LEFT => {
                self.engine.move_cursor_left();
                Effect::Changed(None)
            }
            keys::RIGHT => {
                self.engine.move_cursor_right();
                Effect::Changed(None)
            }
            keys::HOME => {
                self.engine.move_cursor_home();
                Effect::Changed(None)
            }
            keys::END => {
                self.engine.move_cursor_end();
                Effect::Changed(None)
            }
            _ => self.apply_printable(event),
        }
    }

    /// 组句中的可打印键：数字选当前页第 N 个，翻页键对翻页，其余（半角标点等）进英文直输段。
    fn apply_printable(&mut self, event: &KeyEvent) -> Effect {
        if let Some(digit) = keys::digit(event) {
            let page_size = self.config.page_size;
            let page = self.highlight / page_size;
            return Effect::Changed(self.commit_index(page * page_size + digit - 1));
        }
        if let Some(step) = keys::page_key(event, self.config.page_keys) {
            self.page(step);
            return Effect::Navigated;
        }
        match event.character.filter(|c| !c.is_control()) {
            Some(c) => {
                self.engine.push(c);
                Effect::Changed(None)
            }
            None => Effect::Passthrough,
        }
    }

    /// 云联想轮询：只在 `session` 是当前焦点时拉一次异步结果并回最新一帧，否则回空帧。
    /// 释义兜底的结果也借这个节拍收（对应 macOS 壳每秒一次的 `tick`）。
    fn handle_poll(&mut self, session: SessionId) -> ServerMessage {
        let learned = self.engine.poll_glosses();
        if learned > 0 {
            tracing::info!(learned, "释义兜底写入个人释义表");
        }
        let frame = if self.focused == Some(session) {
            self.poll_prediction();
            self.current_frame()
        } else {
            Frame::default()
        };
        ServerMessage::Update { session, frame }
    }

    /// 拉一次云联想结果：云端词并进候选布局的预留槽，整句补全记下。
    fn poll_prediction(&mut self) {
        if !self.engine.prediction_enabled() {
            return;
        }
        let Some(prediction) = self.engine.poll_prediction() else {
            return;
        };
        if let Some(Composed::Candidates { layout, .. }) = self.composed.as_mut() {
            let words: Vec<Candidate> = prediction
                .words
                .into_iter()
                .map(CloudWord::into_candidate)
                .collect();
            layout.set_cloud(words);
            self.sentence = prediction.sentence;
        }
    }

    /// 组句缓冲变化后：按 Engine 状态重建 [`Composed`]，发一次云联想请求，归零高亮与整句补全。
    fn recompose(&mut self) {
        self.highlight = 0;
        self.sentence = None;
        if self.engine.composition().is_empty() {
            self.composed = None;
            self.cancel_prediction();
            return;
        }
        // 先从 query 里把要用的 clone 出来，放开 engine 的借用再发云联想请求。
        let built = self.engine.query().ok().map(|query| {
            let items = query.candidates.items.clone();
            let preedit: Vec<PreeditSegment> =
                query.marked_segments().iter().map(Into::into).collect();
            // 光标要用 Core 的映射：自动补的 `'` 会让显示串比敲的长，各段字符数求和永远等于末尾。
            (items, preedit, query.marked_cursor())
        });
        self.composed = Some(match built {
            Some((items, preedit, cursor)) => {
                let layout =
                    CandidateLayout::new(items, self.config.page_size, self.config.cloud_slots);
                if self.engine.prediction_enabled() {
                    self.engine.request_prediction(None, layout.local());
                }
                Composed::Candidates {
                    preedit,
                    cursor,
                    layout,
                }
            }
            None => {
                self.cancel_prediction();
                let composition = self.engine.composition();
                let text = composition.text().to_owned();
                let cursor = text[..composition.cursor()].chars().count();
                Composed::Raw { text, cursor }
            }
        });
    }

    /// 高亮移动 `delta`，夹在 `[0, 末尾]`，到页边自然换页。
    fn move_highlight(&mut self, delta: isize) {
        let count = self.candidate_count();
        if count == 0 {
            self.highlight = 0;
            return;
        }
        self.highlight = (self.highlight as isize + delta).clamp(0, count as isize - 1) as usize;
    }

    /// 整页翻 `step`，高亮落到目标页第一个候选。
    fn page(&mut self, step: isize) {
        let count = self.candidate_count();
        if count == 0 {
            self.highlight = 0;
            return;
        }
        let page_size = self.config.page_size;
        let page_count = count.div_ceil(page_size);
        let current = (self.highlight / page_size) as isize;
        let target = (current + step).clamp(0, page_count as isize - 1) as usize;
        self.highlight = (target * page_size).min(count - 1);
    }

    /// 候选总数（含已到的云端词）。
    fn candidate_count(&self) -> usize {
        match &self.composed {
            Some(Composed::Candidates { layout, .. }) => layout.len(),
            _ => 0,
        }
    }

    /// 上屏候选布局里第 `index` 个（跨页下标）；没有那个候选就什么都不做。
    fn commit_index(&mut self, index: usize) -> Option<String> {
        let candidate = match &self.composed {
            Some(Composed::Candidates { layout, .. }) => layout.candidate(index).cloned(),
            _ => None,
        }?;
        Some(self.engine.commit(&candidate))
    }

    /// 按当前 [`Composed`] 生成一帧：没在组句给空帧（DLL 收窗口）；正常时给高亮所在的那一页。
    fn current_frame(&self) -> Frame {
        match &self.composed {
            None => Frame::default(),
            Some(Composed::Raw { text, cursor }) => Frame {
                preedit: vec![PreeditSegment {
                    text: text.clone(),
                    kind: PreeditKind::Typed,
                }],
                cursor: *cursor,
                candidates: CandidateList { items: Vec::new() },
                highlight: usize::MAX,
                page: 0,
                page_count: 1,
                layout: self.config.layout,
                theme: self.config.theme,
                sentence: None,
            },
            Some(Composed::Candidates {
                preedit,
                cursor,
                layout,
            }) => {
                let page_size = self.config.page_size;
                // 高亮可能因候选变少而越界，读时夹一下。
                let highlight = self.highlight.min(layout.len().saturating_sub(1));
                let page = highlight / page_size;
                let items: Vec<Candidate> = layout
                    .page(page)
                    .into_iter()
                    .map(|cell| cell.candidate().clone())
                    .collect();
                let mut candidates = CandidateList { items };
                self.engine.annotate(&mut candidates);
                Frame {
                    preedit: preedit.clone(),
                    cursor: *cursor,
                    candidates,
                    highlight: highlight - page * page_size,
                    page,
                    page_count: layout.pages().max(1),
                    layout: self.config.layout,
                    theme: self.config.theme,
                    sentence: self.sentence.clone(),
                }
            }
        }
    }
}
