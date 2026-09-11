//! 协议分派：把 DLL 发来的 [`ClientMessage`] 交给 Engine，产出回给 DLL 的 [`ServerMessage`]。

mod candidates;
mod composed;
mod config;
mod input;
mod keys;
mod reload;
mod session;
mod shortcut;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use qingjian_core::{Candidate, CandidateLayout, CandidateList, CloudWord, Engine};
use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyOutcome, PreeditKind, PreeditSegment, ScreenRect,
    ServerMessage, SessionId,
};

pub use self::candidates::{CandidateSink, NoopSink};
use self::composed::Composed;
pub use self::config::RouterConfig;
use self::session::SessionInfo;

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

    /// 活跃会话及各自的宿主应用。
    sessions: HashMap<SessionId, SessionInfo>,

    /// 当前持有组句的会话。
    focused: Option<SessionId>,

    /// 当前组句的展示状态；没在组句时为 `None`。
    composed: Option<Composed>,

    /// 整句补全：画在 preedit 右侧、Tab 上屏。缓冲变化时清空，异步到达时填。
    sentence: Option<String>,

    /// 当前高亮候选在候选布局里的下标（跨页）。缓冲变化时归 0。
    highlight: usize,

    /// 这轮查询里用方向键 / 翻页键动过高亮。英文模式里空格只在动过之后才选高亮的词，没动过就原样上屏。
    navigated: bool,

    /// 上次把学习数据落盘的时间。
    last_flush: Instant,

    /// 配置热加载：监视 `config.toml` 的 mtime，改动就重新应用；`None` 表示不热加载（如非 Windows 装配测试）。
    reload: Option<reload::ConfigReload>,

    /// 候选窗口的输出端（Server 进程自绘）。缺省空实现，Windows 上由 [`crate::ui`] 注入。
    candidates: Box<dyn CandidateSink>,

    /// 当前聚焦会话最近报来的光标屏幕矩形（[`ClientMessage::PositionCandidates`]）：
    /// 云联想异步到达等没有新按键时，按它原地重摆候选窗口。切走会话 / 组句结束时清。
    last_rect: Option<ScreenRect>,

    /// 上次真正让候选窗口显示的帧与位置：组字期间每 80ms 一次 Poll，多数没变化，
    /// 帧和位置都没变就不重画（省一次分层窗口合成）。收窗口时清。
    last_shown: Option<(Frame, ScreenRect)>,
}

impl Router {
    pub fn new(engine: Engine, config: RouterConfig) -> Self {
        Self {
            engine,
            config: RouterConfig {
                page_size: config.page_size.max(1),
                ..config
            },
            sessions: HashMap::new(),
            focused: None,
            composed: None,
            sentence: None,
            highlight: 0,
            navigated: false,
            last_flush: Instant::now(),
            reload: None,
            candidates: Box::new(NoopSink),
            last_rect: None,
            last_shown: None,
        }
    }

    /// 装候选窗口输出端（Windows 上由 [`crate::ui::CandidateUi`] 注入；不装就是空实现，不画窗口）。
    pub fn set_candidate_sink(&mut self, sink: Box<dyn CandidateSink>) {
        self.candidates = sink;
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
            ClientMessage::OpenSession { session, app } => {
                // 用户要往 [apps] 里加应用时，从这条日志抄 exe 名。
                tracing::debug!(?session, app, "会话打开");
                // 同一会话重开（DLL 断线重连）：清掉可能残留的组句与候选窗口，从干净状态起。
                if self.focused == Some(session) {
                    self.reset_composition();
                    self.focused = None;
                }
                self.sessions.insert(session, SessionInfo { app });
                None
            }
            ClientMessage::Key { session, event } => Some(self.handle_key(session, event)),
            ClientMessage::Poll { session } => Some(self.handle_poll(session)),
            ClientMessage::Commit { session } => {
                let text = self.commit_raw_for(session);
                tracing::debug!(?session, ?text, "焦点离开，结束组句");
                Some(ServerMessage::Committed { session, text })
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
            ClientMessage::PositionCandidates { session, rect } => {
                self.position_candidates(session, rect);
                None
            }
            ClientMessage::HideCandidates { session } => {
                // 组句在 DLL 侧结束、Server 无从知晓（应用强行终止组句）：收起候选窗口。
                // 组句缓冲留着，靠 server_stale 在下一键的 commit 里清（避免把已成文本的拼音重插）。
                if self.focused == Some(session) {
                    self.hide_candidate_window();
                }
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

    /// 当前持有组句的会话所在的应用（exe 名）；没开过会话或 DLL 没报时为 `None`。
    fn focused_app(&self) -> Option<&str> {
        self.focused
            .and_then(|session| self.sessions.get(&session))
            .and_then(|info| info.app.as_deref())
    }

    /// 焦点离开时把缓冲区原样交出（对应 macOS 的 `commitComposition` → `commit_raw`），然后清掉组句状态。
    /// 组句不属于 `session`（别的会话的残留）时只清不交，免得把 A 应用的拼音落进 B 应用。
    fn commit_raw_for(&mut self, session: SessionId) -> Option<String> {
        let text = (self.focused == Some(session) && !self.engine.composition().is_empty())
            .then(|| self.engine.take_raw());
        self.reset_composition();
        text
    }

    /// 清掉当前组句、展示状态与在飞的云联想请求，并收起候选窗口。
    fn reset_composition(&mut self) {
        self.engine.break_chain();
        self.engine.clear();
        self.cancel_prediction();
        self.composed = None;
        self.sentence = None;
        self.highlight = 0;
        self.navigated = false;
        self.hide_candidate_window();
    }

    /// 按一帧和最近的光标矩形调和候选窗口：空帧收窗口并清矩形；非空且已知光标矩形就摆上去重绘；
    /// 非空但还没收到光标矩形（组句刚起、等 [`ClientMessage::PositionCandidates`]）时什么都不做，
    /// 免得先在旧位置闪一下。
    fn reconcile_candidates(&mut self, frame: &Frame) {
        if frame.is_empty() {
            self.hide_candidate_window();
        } else if let Some(rect) = self.last_rect {
            // 帧和位置都没变（组字期间的空转 Poll 很常见）就不重画。
            let unchanged = matches!(&self.last_shown, Some((f, r)) if f == frame && *r == rect);
            if !unchanged {
                self.candidates.show(frame.clone(), rect);
                self.last_shown = Some((frame.clone(), rect));
            }
        }
        // 非空但还没光标矩形（组句刚起）：等 PositionCandidates，先不显示。
    }

    /// 收起候选窗口并清掉定位 / 去抖状态。
    fn hide_candidate_window(&mut self) {
        self.last_rect = None;
        self.last_shown = None;
        self.candidates.hide();
    }

    /// DLL 报来组句范围的屏幕矩形：记下来，按当前帧把候选窗口摆到光标下方。只认聚焦会话。
    fn position_candidates(&mut self, session: SessionId, rect: ScreenRect) {
        if self.focused != Some(session) {
            return;
        }
        self.last_rect = Some(rect);
        let frame = self.current_frame();
        self.reconcile_candidates(&frame);
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
        let frame = self.current_frame();
        // 空帧（选词 / 上屏结束组句）立刻收窗口；组句刚起还没光标矩形时等 PositionCandidates 再摆。
        self.reconcile_candidates(&frame);
        ServerMessage::KeyResult {
            session,
            outcome,
            commit,
            frame,
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
            let frame = self.current_frame();
            // 云端候选 / 整句补全异步到达，没有新按键：按上次的光标矩形原地重摆候选窗口。
            self.reconcile_candidates(&frame);
            frame
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
        self.navigated = false;
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
        let next = (self.highlight as isize + delta).clamp(0, count as isize - 1) as usize;
        self.navigated |= next != self.highlight;
        self.highlight = next;
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
        self.navigated |= target != current as usize;
        self.highlight = (target * page_size).min(count - 1);
    }

    /// 候选总数（含已到的云端词）。
    fn candidate_count(&self) -> usize {
        match &self.composed {
            Some(Composed::Candidates { layout, .. }) => layout.len(),
            _ => 0,
        }
    }

    /// 候选布局里第 `index` 个（跨页下标）。
    fn layout_candidate(&self, index: usize) -> Option<Candidate> {
        match &self.composed {
            Some(Composed::Candidates { layout, .. }) => layout.candidate(index).cloned(),
            _ => None,
        }
    }

    /// 上屏候选布局里第 `index` 个（跨页下标）；没有那个候选就什么都不做。
    fn commit_index(&mut self, index: usize) -> Option<String> {
        let candidate = self.layout_candidate(index)?;
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
