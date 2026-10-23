use std::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::Win32::UI::TextServices::{ITfComposition, ITfContext};

use qingjian_platform::protocol::ScreenRect;

use crate::com::candidate_list::CandidateList;
use super::SelectionAnchor;
use crate::com::service::SharedClient;

/// `TextService`、编辑会话、组句 sink、轮询定时器之间共享的组句状态（STA 单线程，`Rc` 传递）。
pub(crate) struct Shared {
    /// 当前活动的组句，跨按键存活。
    composition: RefCell<Option<ITfComposition>>,

    /// Server 上次回的帧非空；决定 `OnTestKeyDown` 要不要吃功能键。
    composing: Cell<bool>,

    /// 这一段最近报给 Server 的候选窗锚点：之后量不到光标时沿用它，不追着鼠标走。组句结束时清掉。
    anchor: Cell<Option<ScreenRect>>,

    /// 这一段的输入框状态（私密与否、光标前文）已经报给 Server 了。
    /// 行内模式下靠「组句刚起」判断，`preedit = window` 模式应用里没有组句，得另记一个标记。
    context_reported: Cell<bool>,

    /// 「翻译选中文字」评审进行中：所有键交给 Server 定接受 / 取消，轮询定时器照常拉云端译文。
    translating: Cell<bool>,

    selection_anchor: RefCell<Option<SelectionAnchor>>,

    /// 最近一次收键的文档上下文；失焦 / 停用回调不带上下文，落定拼音要用它。
    last_context: RefCell<Option<ITfContext>>,

    /// 组句被应用强行终止过：拼音已成普通文本，但 Server 的缓冲还在，下次说话前先让它清空。
    server_stale: Cell<bool>,

    /// 本线程当前有键盘焦点（`OnSetFocus`）；轮询定时器只在前台时问状态条的切模式请求。
    foreground: Cell<bool>,

    /// 与 `TextService` 共用的引擎客户端；DLL 侧结束组句时要通知 Server 收候选窗口（它无从知晓）。
    client: SharedClient,

    /// 给自己画候选的应用（全屏游戏）的候选列表；应用自己画时不报光标位置，Server 就不画候选窗。
    candidate_list: CandidateList,
}

impl Shared {
    pub(crate) fn new(client: SharedClient) -> Rc<Self> {
        Rc::new(Self {
            composition: RefCell::new(None),
            composing: Cell::new(false),
            anchor: Cell::new(None),
            context_reported: Cell::new(false),
            translating: Cell::new(false),
            selection_anchor: RefCell::new(None),
            last_context: RefCell::new(None),
            server_stale: Cell::new(false),
            foreground: Cell::new(false),
            client,
            candidate_list: CandidateList::default(),
        })
    }

    pub(crate) fn candidate_list(&self) -> &CandidateList {
        &self.candidate_list
    }

    pub(crate) fn foreground(&self) -> bool {
        self.foreground.get()
    }

    pub(crate) fn set_foreground(&self, value: bool) {
        self.foreground.set(value);
    }

    pub(crate) fn last_context(&self) -> Option<ITfContext> {
        self.last_context.borrow().clone()
    }

    pub(crate) fn set_last_context(&self, context: Option<ITfContext>) {
        *self.last_context.borrow_mut() = context;
    }

    pub(crate) fn take_server_stale(&self) -> bool {
        self.server_stale.replace(false)
    }

    pub(crate) fn composing(&self) -> bool {
        self.composing.get()
    }

    pub(crate) fn set_composing(&self, value: bool) {
        self.composing.set(value);
        // 组句结束：下一段重新报输入框状态、重新定锚点。
        if !value {
            self.context_reported.set(false);
            self.anchor.set(None);
        }
    }

    pub(crate) fn anchor(&self) -> Option<ScreenRect> {
        self.anchor.get()
    }

    pub(crate) fn set_anchor(&self, anchor: Option<ScreenRect>) {
        self.anchor.set(anchor);
    }

    pub(crate) fn context_reported(&self) -> bool {
        self.context_reported.get()
    }

    pub(crate) fn set_context_reported(&self, value: bool) {
        self.context_reported.set(value);
    }

    pub(crate) fn translating(&self) -> bool {
        self.translating.get()
    }

    pub(crate) fn set_selection_anchor(&self, anchor: Option<SelectionAnchor>) {
        *self.selection_anchor.borrow_mut() = anchor;
    }

    pub(crate) fn take_selection_anchor(&self) -> Option<SelectionAnchor> {
        self.selection_anchor.borrow_mut().take()
    }

    pub(crate) fn set_translating(&self, value: bool) {
        self.translating.set(value);
    }

    /// 通知 Server 收起候选窗口。引擎正被别处借着或没连上时静默跳过。
    pub(crate) fn hide_candidates(&self) {
        if let Ok(mut guard) = self.client.try_borrow_mut()
            && let Some(client) = guard.as_mut()
        {
            let _ = client.hide_candidates();
        }
    }

    pub(crate) fn has_composition(&self) -> bool {
        self.composition.borrow().is_some()
    }

    /// clone 出来再用，别把 `borrow()` 挂在 match 上（分支里再 `borrow_mut` 会 panic）。
    pub(super) fn composition(&self) -> Option<ITfComposition> {
        self.composition.borrow().clone()
    }

    pub(super) fn set_composition(&self, composition: Option<ITfComposition>) {
        *self.composition.borrow_mut() = composition;
    }

    pub(super) fn take_composition(&self) -> Option<ITfComposition> {
        self.composition.borrow_mut().take()
    }

    /// 组句结束（应用终止组句 / 断线 / 失焦上屏）：不再当作在组句，收起应用的候选列表，并让 Server 收候选窗口。
    pub(crate) fn end_composing(&self) {
        self.set_selection_anchor(None);
        self.composing.set(false);
        self.translating.set(false);
        self.candidate_list.end();
        self.hide_candidates();
    }

    /// 清掉一切本地组句状态，不碰文档。
    pub(crate) fn reset(&self) {
        self.set_composition(None);
        self.set_last_context(None);
        self.end_composing();
    }

    /// 组句被应用强行终止：本地清掉，并记下 Server 的缓冲还没清。
    pub(super) fn terminated(&self) {
        self.reset();
        self.server_stale.set(true);
    }
}
