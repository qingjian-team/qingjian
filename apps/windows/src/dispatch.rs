use std::collections::HashMap;

use qingjian_core::{CandidateList, Engine};
use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyOutcome, PreeditSegment, ServerMessage, SessionId,
};

use crate::session::Session;

/// Windows 虚拟键码：功能键靠它区分（字母 / 数字用 `KeyEvent::character`）。
mod vk {
    pub const BACK: u32 = 0x08;
    pub const RETURN: u32 = 0x0D;
    pub const ESCAPE: u32 = 0x1B;
    pub const SPACE: u32 = 0x20;
}

/// 把 DLL 发来的 [`ClientMessage`] 分派到 Engine，产出要回给 DLL 的 [`ServerMessage`]。
///
/// 同一时刻只有一个应用有键盘焦点，所以像 macOS 那样用**一个** Engine 持当前组句；焦点切到别的会话时
/// 先清掉上一个会话的残留（[`Self::ensure_focus`]）。传输（命名管道）、翻页 / 方向键、英文模式、云联想
/// 与本地整句模型的异步重绘待接。
pub struct Router {
    /// 输入内核，进程内唯一。
    engine: Engine,

    /// 每页候选数。
    page_size: usize,

    /// 活跃会话，按 [`SessionId`] 索引。
    sessions: HashMap<SessionId, Session>,

    /// 当前持有组句的会话；焦点切换时据它判断要不要清空 Engine。
    focused: Option<SessionId>,
}

impl Router {
    pub fn new(engine: Engine, page_size: usize) -> Self {
        Self {
            engine,
            page_size: page_size.max(1),
            sessions: HashMap::new(),
            focused: None,
        }
    }

    /// 处理一条消息；返回 `None` 表示不用回话。
    pub fn handle(&mut self, message: ClientMessage) -> Option<ServerMessage> {
        match message {
            ClientMessage::OpenSession { session } => {
                let state = Session::new(session);
                tracing::debug!(id = ?state.id(), "会话打开");
                self.sessions.insert(session, state);
                None
            }
            ClientMessage::Key { session, event } => Some(self.handle_key(session, event)),
            ClientMessage::Commit { session } => {
                // TODO(windows)：焦点离开时把 preedit 上屏（现在协议没有「无按键的上屏」响应，先丢弃清空）。
                self.engine.break_chain();
                self.engine.clear();
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
                    self.engine.break_chain();
                    self.engine.clear();
                    self.focused = None;
                }
                tracing::debug!(?session, "会话关闭");
                None
            }
        }
    }

    /// 当前活跃会话数。
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// 焦点切到别的会话：清掉上一个会话残留的组句。
    fn ensure_focus(&mut self, session: SessionId) {
        if self.focused != Some(session) {
            self.engine.break_chain();
            self.engine.clear();
            self.focused = Some(session);
        }
    }

    fn handle_key(&mut self, session: SessionId, event: KeyEvent) -> ServerMessage {
        self.ensure_focus(session);
        let composing = !self.engine.composition().is_empty();
        let mut commit = None;
        let consumed;

        if let Some(letter) = event.character.filter(char::is_ascii_alphabetic) {
            // 字母进组句缓冲（英文模式待接）。
            self.engine.push(letter.to_ascii_lowercase());
            consumed = true;
        } else if composing {
            match event.virtual_key {
                vk::BACK => {
                    self.engine.backspace();
                    consumed = true;
                }
                vk::ESCAPE => {
                    self.engine.clear();
                    consumed = true;
                }
                vk::RETURN => {
                    commit = Some(self.engine.take_raw());
                    consumed = true;
                }
                vk::SPACE => {
                    commit = self.commit_index(0);
                    consumed = true;
                }
                _ => match digit(&event) {
                    Some(digit) => {
                        commit = self.commit_index(digit - 1);
                        consumed = true;
                    }
                    None => consumed = false,
                },
            }
        } else {
            // 没在组句、又不是字母：交还给应用。
            consumed = false;
        }

        let outcome = if consumed {
            KeyOutcome::Consumed
        } else {
            KeyOutcome::Passthrough
        };
        ServerMessage::KeyResult {
            session,
            outcome,
            commit,
            frame: self.current_frame(),
        }
    }

    /// 上屏当前候选页第 `index` 个候选（页内下标，从 0 起）；没有那个候选就什么都不做。
    fn commit_index(&mut self, index: usize) -> Option<String> {
        let query = self.engine.query().ok()?;
        let candidate = query.candidates.items.get(index)?.clone();
        Some(self.engine.commit(&candidate))
    }

    /// 按当前组句状态生成一帧：没有在组句返回空帧（DLL 收窗口）。
    fn current_frame(&self) -> Frame {
        if self.engine.composition().is_empty() {
            return Frame::default();
        }
        let Ok(query) = self.engine.query() else {
            return Frame::default();
        };
        let total = query.candidates.items.len();
        let items = query
            .candidates
            .items
            .iter()
            .take(self.page_size)
            .cloned()
            .collect();
        let mut candidates = CandidateList { items };
        self.engine.annotate(&mut candidates);
        let preedit: Vec<PreeditSegment> = query.marked_segments().iter().map(Into::into).collect();
        let cursor = preedit.iter().map(|s| s.text.chars().count()).sum();
        Frame {
            preedit,
            cursor,
            candidates,
            highlight: 0,
            page: 0,
            page_count: total.div_ceil(self.page_size).max(1),
        }
    }
}

/// 数字键 1–9 → 页内下标 1–9（`character` 优先，退回虚拟键码 0x31–0x39）。
fn digit(event: &KeyEvent) -> Option<usize> {
    if let Some(c) = event.character.filter(|c| ('1'..='9').contains(c)) {
        return Some(c as usize - '0' as usize);
    }
    (0x31..=0x39)
        .contains(&event.virtual_key)
        .then(|| (event.virtual_key - 0x30) as usize)
}
