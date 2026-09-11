use std::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::Win32::UI::TextServices::{ITfComposition, ITfContext};

use qingjian_platform::protocol::Frame;

use crate::com::candidates::CandidateWindow;

/// `TextService`、编辑会话、组句 sink、轮询定时器之间共享的组句状态。STA 单线程，用 `Rc` 传递。
pub(crate) struct Shared {
    /// 当前活动的组句，跨按键存活。
    composition: RefCell<Option<ITfComposition>>,

    /// 组句影子标志：Server 上次回的帧空不空，决定 `OnTestKeyDown` 要不要吃功能键。
    composing: Cell<bool>,

    /// 最近一次收键的文档上下文。失焦 / 停用回调不带上下文，要把拼音落进文档得用它。
    last_context: RefCell<Option<ITfContext>>,

    /// 组句被应用强行终止过：拼音已被框架定成普通文本，但 Server 的缓冲还在，下次和 Server 说话前先让它清空。
    server_stale: Cell<bool>,

    /// 候选窗口；建失败时为 `None`，降级为无候选 UI。
    window: RefCell<Option<CandidateWindow>>,

    /// 上次灌进候选窗口的一帧，轮询据它判断有没有变化。
    last_frame: RefCell<Option<Frame>>,
}

impl Shared {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self {
            composition: RefCell::new(None),
            composing: Cell::new(false),
            last_context: RefCell::new(None),
            server_stale: Cell::new(false),
            window: RefCell::new(None),
            last_frame: RefCell::new(None),
        })
    }

    pub(crate) fn last_context(&self) -> Option<ITfContext> {
        self.last_context.borrow().clone()
    }

    pub(crate) fn set_last_context(&self, context: Option<ITfContext>) {
        *self.last_context.borrow_mut() = context;
    }

    /// 取走「Server 缓冲已过期」标志。
    pub(crate) fn take_server_stale(&self) -> bool {
        self.server_stale.replace(false)
    }

    pub(crate) fn composing(&self) -> bool {
        self.composing.get()
    }

    pub(crate) fn set_composing(&self, value: bool) {
        self.composing.set(value);
    }

    /// 是否还有一个活动组句句柄没收。
    pub(crate) fn has_composition(&self) -> bool {
        self.composition.borrow().is_some()
    }

    /// 当前活动组句的一份句柄。clone 出来再用，别把 `borrow()` 挂在 match 上（分支里再 `borrow_mut` 会 panic）。
    pub(super) fn composition(&self) -> Option<ITfComposition> {
        self.composition.borrow().clone()
    }

    pub(super) fn set_composition(&self, composition: Option<ITfComposition>) {
        *self.composition.borrow_mut() = composition;
    }

    pub(super) fn take_composition(&self) -> Option<ITfComposition> {
        self.composition.borrow_mut().take()
    }

    /// 装 / 卸候选窗口。传 `None` 会析构原窗口。
    pub(crate) fn set_window(&self, window: Option<CandidateWindow>) {
        *self.window.borrow_mut() = window;
    }

    /// 对候选窗口做点什么（没有窗口时跳过）。
    pub(super) fn with_window(&self, f: impl FnOnce(&CandidateWindow)) {
        if let Some(window) = self.window.borrow().as_ref() {
            f(window);
        }
    }

    /// 用一帧刷新候选窗口内容。不定位、不显示：那要光标位置，在编辑会话里做。
    pub(crate) fn update_candidates(&self, frame: &Frame) {
        self.with_window(|window| window.set_content(frame));
        *self.last_frame.borrow_mut() = Some(frame.clone());
    }

    /// 轮询到一帧：和上次比，变了才灌进候选窗口并按上次光标位置原地重绘。
    pub(crate) fn apply_poll(&self, frame: &Frame) {
        if self.last_frame.borrow().as_ref() == Some(frame) {
            return;
        }
        self.update_candidates(frame);
        self.with_window(CandidateWindow::refresh);
    }

    /// Server 侧组句已结束（断线 / 失焦上屏）：不再当作在组句（否则功能键会一直被吃）、收起候选窗口。
    /// 组句句柄留着，由下一次编辑会话收掉。
    pub(crate) fn end_composing(&self) {
        self.composing.set(false);
        *self.last_frame.borrow_mut() = None;
        self.with_window(CandidateWindow::hide);
    }

    /// 清掉一切本地组句状态（停用 / 组句被应用强行终止时）：丢组句句柄与上下文、收起候选窗口。不碰文档。
    pub(crate) fn reset(&self) {
        self.set_composition(None);
        self.set_last_context(None);
        self.end_composing();
    }

    /// 组句被应用强行终止：本地状态清掉，并记下 Server 的缓冲还没清。
    pub(super) fn terminated(&self) {
        self.reset();
        self.server_stale.set(true);
    }
}
