//! 文本服务对象 [`TextService`]：每线程一个，实现 `ITfTextInputProcessor`（激活 / 停用，[`processor`]）、
//! `ITfKeyEventSink`（收键，[`key_sink`]）与显示属性提供者（[`display`]）。
//! 连 Server 在 [`connection`]，中英模式在 [`mode`]，往文档写字在 [`document`]。

mod connection;
mod display;
mod document;
mod key_sink;
mod mode;
mod next;
mod processor;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use windows::Win32::UI::TextServices::{
    ITfDisplayAttributeProvider, ITfKeyEventSink, ITfLangBarItemButton, ITfSource,
    ITfTextInputProcessor, ITfThreadMgr,
};
use windows::core::{ComObject, implement};

use qingjian_platform::KeyCombo;

use super::composition::Shared;
use super::key::ShiftTap;
use super::mode::ModeState;
use super::poll::PollTimer;
use crate::client::EngineClient;
use crate::client::pipe::PipeStream;

/// 连 Server 的会话客户端，与编辑会话 / 轮询定时器共享（STA 单线程）。连不上时为 `None`，键照样放行。
pub(crate) type SharedClient = Rc<RefCell<Option<EngineClient<PipeStream>>>>;

/// 连不上 Server 后隔多久再试（每次尝试都在应用的 UI 线程上，不能每键都试）。
const RECONNECT_INTERVAL: Duration = Duration::from_secs(2);

/// 一个 TSF 文本服务实例（每线程一个）。
#[implement(ITfTextInputProcessor, ITfKeyEventSink, ITfDisplayAttributeProvider)]
pub struct TextService {
    /// 激活时拿到的线程管理器，停用时用它反注册。
    thread_mgr: RefCell<Option<ITfThreadMgr>>,

    /// TSF 分配的 client id。
    client_id: Cell<u32>,

    /// 引擎层。
    engine: SharedClient,

    /// 跨按键存活的组句状态。
    shared: Rc<Shared>,

    /// 云联想轮询定时器；挂失败时为 `None`，退化为只在按键时收云结果。
    poll_timer: RefCell<Option<PollTimer>>,

    /// 上次连 Server 失败的时间，按 [`RECONNECT_INTERVAL`] 退避。
    last_connect_failure: Cell<Option<Instant>>,

    /// 中 / 英模式（单击 Shift 翻转），与语言栏按钮共用。
    mode_state: Rc<ModeState>,

    /// 登记在系统语言栏上的中 / 英按钮；停用时反注册。
    mode_button: RefCell<Option<ITfLangBarItemButton>>,

    /// 「转换模式」compartment 的事件回调（source + cookie），反向同步任务栏点选；停用时撤掉。
    conversion_sink: RefCell<Option<(ITfSource, u32)>>,

    /// 单击 Shift 切中英的判定。
    shift_tap: ShiftTap,

    /// 语言 profile 通知挂上后的 cookie；挂一次就够（见 [`super::profile`]）。
    profile_cookie: Cell<Option<u32>>,

    /// 登记成保留键的「翻译选中文字」组合；停用时撤掉（见 [`preserved`](crate::com::key::preserved)）。
    translate_combo: Cell<Option<KeyCombo>>,
}

thread_local! {
    /// 本线程当前激活的文本服务，供转换模式回调 / 轮询定时器切模式。`Activate` 设、`Deactivate` 清。
    static ACTIVE: RefCell<Option<ComObject<TextService>>> = const { RefCell::new(None) };
}

fn with_active(f: impl FnOnce(&TextService_Impl)) {
    let service = ACTIVE.with(|active| active.borrow().clone());
    if let Some(service) = service {
        f(&service);
    }
}

/// 用户点了语言栏的中 / 英按钮（见 [`ModeButton`](crate::com::mode::ModeButton)）：翻转模式。
pub(super) fn toggle_mode() {
    with_active(|service| service.set_english_mode(!service.mode_state.english()));
}

/// 「转换模式」compartment 变了（见 [`conversion`](crate::com::mode::conversion)）。
pub(super) fn on_conversion_mode_changed() {
    with_active(TextService_Impl::sync_from_conversion_mode);
}

/// 轮询取到了状态条上点出的目标模式（见 [`super::poll`]）；与当前相同就不动。
pub(super) fn on_mode_sync(english: bool) {
    with_active(|service| {
        if service.mode_state.english() != english {
            service.set_english_mode(english);
        }
    });
}

impl TextService {
    #[allow(clippy::new_without_default)] // 有 lock_module 副作用
    pub fn new() -> Self {
        crate::com::lock_module();
        let engine: SharedClient = Rc::new(RefCell::new(None));
        Self {
            thread_mgr: RefCell::new(None),
            client_id: Cell::new(0),
            shared: Shared::new(engine.clone()),
            engine,
            poll_timer: RefCell::new(None),
            last_connect_failure: Cell::new(None),
            mode_state: ModeState::new(),
            mode_button: RefCell::new(None),
            conversion_sink: RefCell::new(None),
            shift_tap: ShiftTap::default(),
            profile_cookie: Cell::new(None),
            translate_combo: Cell::new(None),
        }
    }
}

impl Drop for TextService {
    fn drop(&mut self) {
        crate::com::unlock_module();
    }
}
