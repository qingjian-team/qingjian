use std::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::Win32::UI::TextServices::{ITfComposition, ITfContext};

use crate::com::service::SharedClient;

/// `TextService`、编辑会话、组句 sink、轮询定时器之间共享的组句状态。STA 单线程，用 `Rc` 传递。
///
/// 候选窗口在 Server 进程自绘，DLL 这边只管 preedit 内联与光标上报，所以这里不再持有窗口 / 候选帧。
pub(crate) struct Shared {
    /// 当前活动的组句，跨按键存活。
    composition: RefCell<Option<ITfComposition>>,

    /// 组句影子标志：Server 上次回的帧空不空，决定 `OnTestKeyDown` 要不要吃功能键。
    composing: Cell<bool>,

    /// 最近一次收键的文档上下文。失焦 / 停用回调不带上下文，要把拼音落进文档得用它。
    last_context: RefCell<Option<ITfContext>>,

    /// 组句被应用强行终止过：拼音已被框架定成普通文本，但 Server 的缓冲还在，下次和 Server 说话前先让它清空。
    server_stale: Cell<bool>,

    /// 引擎层客户端（与 `TextService` 共用同一个 `Rc`）。组句在 DLL 侧结束时（应用终止组句等）用它发
    /// `HideCandidates` 让 Server 收起候选窗口——候选窗口在 Server 进程自绘，Server 无从知晓 DLL 侧的组句结束。
    client: RefCell<Option<SharedClient>>,
}

impl Shared {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self {
            composition: RefCell::new(None),
            composing: Cell::new(false),
            last_context: RefCell::new(None),
            server_stale: Cell::new(false),
            client: RefCell::new(None),
        })
    }

    /// 装引擎层客户端（激活时调）。传的是 `TextService::engine` 的克隆，同一个连接。
    pub(crate) fn set_client(&self, client: SharedClient) {
        *self.client.borrow_mut() = Some(client);
    }

    /// 通知 Server 收起候选窗口。引擎正被别处借着（如收键中出错）或没连上时静默跳过。
    fn hide_server_candidates(&self) {
        if let Some(client) = self.client.borrow().as_ref()
            && let Ok(mut guard) = client.try_borrow_mut()
            && let Some(client) = guard.as_mut()
        {
            let _ = client.hide_candidates();
        }
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

    /// 组句结束（应用终止组句 / 断线 / 失焦上屏）：不再当作在组句（否则功能键会一直被吃），
    /// 并通知 Server 收起候选窗口——应用终止组句这条路径 Server 无从知晓，不发它候选窗会一直挂着。
    pub(crate) fn end_composing(&self) {
        self.composing.set(false);
        self.hide_server_candidates();
    }

    /// 清掉一切本地组句状态（停用 / 组句被应用强行终止时）：丢组句句柄与上下文。不碰文档。
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
