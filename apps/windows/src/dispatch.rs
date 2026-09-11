use std::collections::HashMap;

use qingjian_core::{CandidateList, Engine};
use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyOutcome, PreeditKind, PreeditSegment, ServerMessage,
    SessionId,
};
use qingjian_platform::{LayoutMode, ThemeMode};

use crate::session::Session;

/// Windows 虚拟键码：功能键靠它区分（字母 / 数字用 `KeyEvent::character`）。
mod vk {
    pub const BACK: u32 = 0x08;
    pub const RETURN: u32 = 0x0D;
    pub const ESCAPE: u32 = 0x1B;
    pub const SPACE: u32 = 0x20;
    pub const PRIOR: u32 = 0x21; // PageUp
    pub const NEXT: u32 = 0x22; // PageDown
    pub const END: u32 = 0x23;
    pub const HOME: u32 = 0x24;
    pub const LEFT: u32 = 0x25;
    pub const UP: u32 = 0x26;
    pub const RIGHT: u32 = 0x27;
    pub const DOWN: u32 = 0x28;
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

    /// 当前高亮候选在**整个候选列表**里的下标（跨页）。组句缓冲变化（敲字 / 退格 / 移光标）时归 0，
    /// 上下键移动、翻页键整页跳。页码由它与 `page_size` 推出。
    highlight: usize,

    /// 候选排布（竖排 / 横排），由 `[general] layout` 配置决定，随每帧下发给 DLL 渲染。
    layout: LayoutMode,

    /// 候选窗口外观（跟随系统 / 浅色 / 深色），由 `[general] theme` 配置决定，随每帧下发；`System` 由 DLL 解析。
    theme: ThemeMode,
}

impl Router {
    pub fn new(engine: Engine, page_size: usize, layout: LayoutMode, theme: ThemeMode) -> Self {
        Self {
            engine,
            page_size: page_size.max(1),
            sessions: HashMap::new(),
            focused: None,
            highlight: 0,
            layout,
            theme,
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
            // 字母进组句缓冲（英文模式待接）；新键让高亮回第一个。
            self.engine.push(letter.to_ascii_lowercase());
            self.highlight = 0;
            consumed = true;
        } else if composing {
            // 组句中：功能键导航 / 翻页 / 移光标 / 选词，半角标点进英文直输段（Engine::push）。
            match event.virtual_key {
                vk::BACK => {
                    self.engine.backspace();
                    self.highlight = 0;
                    consumed = true;
                }
                vk::ESCAPE => {
                    self.engine.clear();
                    self.highlight = 0;
                    consumed = true;
                }
                vk::RETURN => {
                    commit = Some(self.engine.take_raw());
                    consumed = true;
                }
                vk::SPACE => {
                    commit = self.commit_index(self.highlight);
                    consumed = true;
                }
                vk::DOWN => {
                    self.move_highlight(1);
                    consumed = true;
                }
                vk::UP => {
                    self.move_highlight(-1);
                    consumed = true;
                }
                vk::NEXT => {
                    self.page(1);
                    consumed = true;
                }
                vk::PRIOR => {
                    self.page(-1);
                    consumed = true;
                }
                vk::LEFT => {
                    self.engine.move_cursor_left();
                    self.highlight = 0;
                    consumed = true;
                }
                vk::RIGHT => {
                    self.engine.move_cursor_right();
                    self.highlight = 0;
                    consumed = true;
                }
                vk::HOME => {
                    self.engine.move_cursor_home();
                    self.highlight = 0;
                    consumed = true;
                }
                vk::END => {
                    self.engine.move_cursor_end();
                    self.highlight = 0;
                    consumed = true;
                }
                _ => match digit(&event) {
                    // 数字选当前页第 N 个（页内下标 → 整个列表下标）。
                    Some(digit) => {
                        let page = self.highlight / self.page_size;
                        commit = self.commit_index(page * self.page_size + digit - 1);
                        consumed = true;
                    }
                    None => match page_key(&event) {
                        // 翻页键对（缺省 [ ]）。
                        Some(step) => {
                            self.page(step);
                            consumed = true;
                        }
                        // 其余可打印字符（半角标点等）进英文直输段。
                        None => match event.character.filter(|c| !c.is_control()) {
                            Some(c) => {
                                self.engine.push(c);
                                self.highlight = 0;
                                consumed = true;
                            }
                            None => consumed = false,
                        },
                    },
                },
            }
        } else {
            // 没在组句、又不是字母：交还给应用。
            consumed = false;
        }

        // 上屏后组句结束，高亮归零，下一段从头开始。
        if commit.is_some() {
            self.highlight = 0;
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

    /// 高亮在整个候选列表里移动 `delta`（+1 下一个 / -1 上一个），夹在 `[0, 末尾]`，到页边自然换页。
    fn move_highlight(&mut self, delta: isize) {
        let count = self.candidate_count();
        if count == 0 {
            self.highlight = 0;
            return;
        }
        let last = count - 1;
        self.highlight = (self.highlight as isize + delta).clamp(0, last as isize) as usize;
    }

    /// 整页翻 `step`（+1 下一页 / -1 上一页）：高亮落到目标页的第一个候选。
    fn page(&mut self, step: isize) {
        let count = self.candidate_count();
        if count == 0 {
            self.highlight = 0;
            return;
        }
        let page_count = count.div_ceil(self.page_size);
        let current = (self.highlight / self.page_size) as isize;
        let target = (current + step).clamp(0, page_count as isize - 1) as usize;
        self.highlight = (target * self.page_size).min(count - 1);
    }

    /// 当前组句的候选总数（查询失败当 0）。
    fn candidate_count(&self) -> usize {
        self.engine
            .query()
            .map(|query| query.candidates.items.len())
            .unwrap_or(0)
    }

    /// 上屏候选列表里第 `index` 个候选（**整个列表**下标，从 0 起）；没有那个候选就什么都不做。
    fn commit_index(&mut self, index: usize) -> Option<String> {
        let query = self.engine.query().ok()?;
        let candidate = query.candidates.items.get(index)?.clone();
        Some(self.engine.commit(&candidate))
    }

    /// 按当前组句状态生成一帧：没有在组句返回空帧（DLL 收窗口）。显示高亮所在的那一页，`highlight` 换算成
    /// 页内下标。**查询失败（整段切不动，如中途上屏后剩下不可切分的残余）时退回显示原始拼音**——对齐 macOS
    /// 端 `refresh`，绝不能返回空帧：组句非空却空帧会让候选窗消失、组句卡死到只有 clear / 切输入法能解。
    fn current_frame(&self) -> Frame {
        let composition = self.engine.composition();
        if composition.is_empty() {
            return Frame::default();
        }
        let Ok(query) = self.engine.query() else {
            let text = composition.text().to_owned();
            let cursor = text[..composition.cursor()].chars().count();
            return Frame {
                preedit: vec![PreeditSegment {
                    text,
                    kind: PreeditKind::Typed,
                }],
                cursor,
                candidates: CandidateList { items: Vec::new() },
                highlight: usize::MAX,
                page: 0,
                page_count: 1,
                layout: self.layout,
                theme: self.theme,
            };
        };
        let total = query.candidates.items.len();
        let page_count = total.div_ceil(self.page_size).max(1);
        // 高亮可能因候选变少而越界，读时夹一下。
        let highlight = self.highlight.min(total.saturating_sub(1));
        let page = highlight / self.page_size;
        let start = page * self.page_size;
        let items = query
            .candidates
            .items
            .iter()
            .skip(start)
            .take(self.page_size)
            .cloned()
            .collect();
        let mut candidates = CandidateList { items };
        self.engine.annotate(&mut candidates);
        let preedit: Vec<PreeditSegment> = query.marked_segments().iter().map(Into::into).collect();
        // 光标在 marked text 里的字符位置：用 Core 现成的映射（自动补的 `'` 会让显示串比敲的长），
        // 不能拿各段字符数求和——那永远等于末尾，左右移光标时光标不动（对齐 macOS `refresh`）。
        let cursor = query.marked_cursor();
        Frame {
            preedit,
            cursor,
            candidates,
            highlight: highlight - start,
            page,
            page_count,
            layout: self.layout,
            theme: self.theme,
        }
    }
}

/// 翻页键对（缺省 `[` 上一页 / `]` 下一页）：返回 -1 / +1，其余 `None`。以后由 `[general] page_keys` 配置。
fn page_key(event: &KeyEvent) -> Option<isize> {
    match event.character {
        Some('[') => Some(-1),
        Some(']') => Some(1),
        _ => None,
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
