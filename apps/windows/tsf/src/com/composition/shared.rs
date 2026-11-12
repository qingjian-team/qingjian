use std::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::Win32::UI::TextServices::{ITfComposition, ITfContext};

use qingjian_platform::protocol::{Frame, ScreenRect};

use crate::com::candidate_list::CandidateList;
use crate::com::service::SharedClient;

/// `TextService`、编辑会话、组句 sink、轮询定时器之间共享的组句状态（STA 单线程，`Rc` 传递）。
pub(crate) struct Shared {
    /// 当前活动的组句，跨按键存活。
    composition: RefCell<Option<ITfComposition>>,

    /// 最近一次写进文档的组句文字；符号面板翻译组的译文靠轮询送来，对比它才知道要不要补写。
    preedit: RefCell<String>,

    /// Server 上次回的帧非空；决定 `OnTestKeyDown` 要不要吃功能键。
    composing: Cell<bool>,

    /// 这一段最近报给 Server 的候选窗锚点：之后量不到光标时沿用它，不追着鼠标走。组句结束时清掉。
    anchor: Cell<Option<ScreenRect>>,

    /// 这一段的输入框状态（私密与否、光标前文）已经报给 Server 了。
    /// 行内模式下靠「组句刚起」判断，`preedit = window` 模式应用里没有组句，得另记一个标记。
    context_reported: Cell<bool>,

    /// Server 上次回的帧说面板停在翻译框（[`Frame::panel_text_box`]）：Ctrl+A 交给 Server 全选框里的字。
    text_box: Cell<bool>,

    /// 「翻译选中文字」评审进行中：所有键交给 Server 定接受 / 取消，轮询定时器照常拉云端译文。
    translating: Cell<bool>,

    /// 最近一次收键的文档上下文；失焦 / 停用回调不带上下文，落定拼音要用它。
    last_context: RefCell<Option<ITfContext>>,

    /// 组句被应用强行终止过：拼音已成普通文本，但 Server 的缓冲还在，下次说话前先让它清空。
    server_stale: Cell<bool>,

    /// 本线程当前有键盘焦点（`OnSetFocus`）；轮询定时器只在前台时问状态条的切模式请求。
    foreground: Cell<bool>,

    /// 给 Chromium 系应用挪光标时模拟的 Shift 事件还有几个没经过击键 sink；它们不算用户按的，单击 Shift 的判定要跳过。
    injected_shift: Cell<u32>,

    /// 与 `TextService` 共用的引擎客户端；DLL 侧结束组句时要通知 Server 收候选窗口（它无从知晓）。
    client: SharedClient,

    /// 给自己画候选的应用（全屏游戏）的候选列表；应用自己画时不报光标位置，Server 就不画候选窗。
    candidate_list: CandidateList,
}

impl Shared {
    pub(crate) fn new(client: SharedClient) -> Rc<Self> {
        Rc::new(Self {
            composition: RefCell::new(None),
            preedit: RefCell::new(String::new()),
            composing: Cell::new(false),
            anchor: Cell::new(None),
            context_reported: Cell::new(false),
            text_box: Cell::new(false),
            translating: Cell::new(false),
            last_context: RefCell::new(None),
            server_stale: Cell::new(false),
            foreground: Cell::new(false),
            injected_shift: Cell::new(0),
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

    pub(crate) fn preedit(&self) -> String {
        self.preedit.borrow().clone()
    }

    pub(crate) fn set_preedit(&self, text: &str) {
        text.clone_into(&mut self.preedit.borrow_mut());
    }

    pub(crate) fn composing(&self) -> bool {
        self.composing.get()
    }

    pub(crate) fn set_composing(&self, value: bool) {
        self.composing.set(value);
        // 组句结束：下一段重新报输入框状态、重新定锚点。
        if !value {
            self.text_box.set(false);
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

    /// 按 Server 回的帧记下在不在组句、面板是不是停在翻译框。
    pub(crate) fn follow_frame(&self, frame: &Frame) {
        self.set_composing(!frame.is_empty());
        self.text_box.set(frame.panel_text_box);
    }

    pub(crate) fn text_box(&self) -> bool {
        self.text_box.get()
    }

    pub(crate) fn translating(&self) -> bool {
        self.translating.get()
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

    pub(super) fn ignore_shift_events(&self, count: u32) {
        self.injected_shift.set(self.injected_shift.get() + count);
    }

    /// 失焦时清掉：模拟的 Shift 没回到本线程（焦点刚好切走），留着会吞掉用户之后真按的一次。
    pub(crate) fn clear_injected_shift(&self) {
        self.injected_shift.set(0);
    }

    /// 这个 Shift 事件是模拟的：消耗一个计数并返回 `true`。
    pub(crate) fn take_injected_shift(&self) -> bool {
        let left = self.injected_shift.get();
        self.injected_shift.set(left.saturating_sub(1));
        left > 0
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
        self.composing.set(false);
        self.text_box.set(false);
        self.translating.set(false);
        self.candidate_list.end();
        self.hide_candidates();
    }

    /// 清掉一切本地组句状态，不碰文档。
    pub(crate) fn reset(&self) {
        self.set_composition(None);
        self.set_preedit("");
        self.set_last_context(None);
        self.end_composing();
    }

    /// 组句被应用强行终止：本地清掉，并记下 Server 的缓冲还没清。
    pub(super) fn terminated(&self) {
        self.reset();
        self.server_stale.set(true);
    }
}
