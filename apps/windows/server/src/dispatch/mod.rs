//! 协议分派：把 DLL 发来的 [`ClientMessage`] 交给 Engine，产出回给 DLL 的 [`ServerMessage`]。
//! 消息分派在 [`message`]，会话在 [`session`]，组句展示状态在 [`composed`]，按键在 [`key`]，
//! 候选窗口输出在 [`candidates`]，状态条在 [`status`]，翻译选中文字在 [`translate`]，配置热加载在 [`reload`]，
//! 本地整句模型在 [`rescore`]，形码码表在 [`code`]。

mod candidates;
mod code;
mod composed;
mod config;
mod key;
mod message;
mod mode;
mod reload;
mod rescore;
mod session;
mod status;
mod translate;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use qingjian_core::Engine;
use qingjian_platform::LocalModelConfig;
use qingjian_platform::protocol::{
    ClientMessage, Frame, IndicatorState, InputSettings, ScreenRect, ServerMessage, SessionId,
};

pub use self::candidates::{CandidateSink, NoopSink, RenderSettings};
pub use self::code::find_code_table;
use self::composed::Composed;
pub use self::config::RouterConfig;
use self::reload::ConfigReload;
pub use self::reload::{DataDirs, attach_cloud};
pub use self::rescore::find_model;
use self::rescore::{ModelLoader, RescoreState};
use self::session::SessionInfo;
pub use self::status::{NoopStatusSink, StatusEvent, StatusSink, StatusView};
use self::translate::Translation;

/// 学习数据落盘间隔（与 macOS 壳一致）；Server 没有定时器，借消息节拍看时间。
const LEARNING_FLUSH_INTERVAL: Duration = Duration::from_secs(60);

/// 同一时刻只有一个应用有键盘焦点，所以一个 Engine 持当前组句；焦点切到别的会话时先清掉上一个的残留。
pub struct Router {
    /// 输入内核，进程内唯一。
    engine: Engine,

    /// 每页候选数 / 云端槽位 / 排布 / 外观 / 翻页键等。
    config: RouterConfig,

    /// 活跃会话及各自的宿主应用。
    sessions: HashMap<SessionId, SessionInfo>,

    /// 当前持有组句的会话。
    focused: Option<SessionId>,

    /// 当前组句的展示状态；没在组句时为 `None`。
    composed: Option<Composed>,

    /// 「翻译选中文字」进行态；与 `composed` 互斥。
    translation: Option<Translation>,

    /// 已发出、等 DLL 回选区的请求号；对不上的 `Selection` 丢弃。
    pending_selection: Option<u64>,

    /// 「翻译选中文字」请求号计数器。
    selection_seq: u64,

    /// 整句补全（preedit 右侧、Tab 上屏）；缓冲变化时清空。
    sentence: Option<String>,

    /// 删候选后的屏幕提示，随下一帧下发、下一次按键清。
    notice: Option<String>,

    /// 当前高亮候选在布局里的下标（跨页）。
    highlight: usize,

    /// 这轮查询里动过高亮：动过就不再拿重排结果换掉候选；注音模式数字键动过之后才选词。
    navigated: bool,

    /// 上次把学习数据落盘的时间。
    last_flush: Instant,

    /// 配置热加载状态；`None` 表示不热加载。
    reload: Option<ConfigReload>,

    /// 候选窗口输出端；Windows 上由 [`crate::ui`] 注入。
    candidates: Box<dyn CandidateSink>,

    /// 悬浮状态条输出端；Windows 上由 [`crate::ui`] 注入。
    status: Box<dyn StatusSink>,

    /// 当前中英模式（`true` 英文）；按应用记忆时是 `mode_session` 对应的那份。
    english: bool,

    /// 最近同步中英状态的会话，与持有组句的 `focused` 分开，切模式不抢走待上屏的组句。
    mode_session: Option<SessionId>,

    /// 应用 exe 名（小写）到中英状态；会话断开仍保留，Server 重启后清空。
    app_modes: HashMap<String, bool>,

    /// 当前输入法是不是青简：有 DLL 来取模式就是，切成别的输入法时收起。状态条只在这时显示；
    /// 应用退出不影响它，状态条是桌面常驻的。
    ime_active: bool,

    /// 聚焦会话最近报来的光标矩形；云联想异步到达时按它原地重摆候选窗口。
    last_rect: Option<ScreenRect>,

    /// 上次真正显示的帧与位置：没变就不重画（组字期间的空转 Poll 很多）。
    last_shown: Option<(Frame, ScreenRect)>,

    /// 本地整句模型（`.qjm` 或三件套目录）；没有模型文件为 `None`。
    model_path: Option<PathBuf>,

    /// 形码码表（`wubi/wubi86.tsv`，启动时找好的，见 [`code::find_code_table`]）；没有为 `None`。
    code_table: Option<PathBuf>,

    /// 进行中的模型加载；加载完接到 Engine 上就清掉。
    model_loader: Option<ModelLoader>,

    /// 上次套用的 `[model]`，变了才重载 / 卸载。
    applied_model: LocalModelConfig,

    /// 重排的防抖 / 轮询进行态。
    rescore: RescoreState,
}

impl Router {
    pub fn new(engine: Engine, config: RouterConfig) -> Self {
        let english = config.english_mode && config.default_english;
        Self {
            engine,
            config: RouterConfig {
                page_size: config.page_size.max(1),
                ..config
            },
            sessions: HashMap::new(),
            focused: None,
            composed: None,
            translation: None,
            pending_selection: None,
            selection_seq: 0,
            sentence: None,
            notice: None,
            highlight: 0,
            navigated: false,
            last_flush: Instant::now(),
            reload: None,
            candidates: Box::new(NoopSink),
            status: Box::new(NoopStatusSink),
            english,
            mode_session: None,
            app_modes: HashMap::new(),
            ime_active: false,
            last_rect: None,
            last_shown: None,
            model_path: None,
            code_table: None,
            model_loader: None,
            applied_model: LocalModelConfig::default(),
            rescore: RescoreState::default(),
        }
    }

    /// 下发给 DLL 的按键行为设置：`OpenSession` 的回包带一次，之后每拍 `SyncMode` 也跟着走，
    /// 所以 DLL 不用自己读配置文件，配置改了也不用重开会话。
    pub(super) fn input_settings(&self) -> InputSettings {
        InputSettings {
            switch_mode: self.config.switch_mode,
            english_mode: self.config.english_mode,
            shift_letter_compose: self.config.shift_letter_compose,
            restore_mode: self.config.default_english || self.config.remember_mode_per_app,
        }
    }

    /// 任务栏图标右键菜单打勾用的开关状态，随 `ModeSync` 每一拍下发。
    pub(super) fn indicator_state(&self) -> IndicatorState {
        IndicatorState {
            full_width_punctuation: self.config.full_width,
            english_full_width_punctuation: self.config.english_full_width,
            status_bar: self.config.status_enabled,
            update_available: self.update_available(),
        }
    }

    pub fn set_candidate_sink(&mut self, sink: Box<dyn CandidateSink>) {
        sink.configure(self.config.render_settings());
        self.candidates = sink;
    }

    /// 直接碰 Engine：测试里改模式键这类启动时才设的开关。
    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }

    pub fn set_status_sink(&mut self, sink: Box<dyn StatusSink>) {
        self.status = sink;
    }

    /// 处理一条消息；`None` 表示不用回话。到点顺带把学习数据落盘。
    pub fn handle(&mut self, message: ClientMessage) -> Option<ServerMessage> {
        let response = self.dispatch(message);
        if self.last_flush.elapsed() >= LEARNING_FLUSH_INTERVAL {
            self.flush_learning();
        }
        response
    }

    pub fn flush_learning(&mut self) {
        self.engine.flush_learning();
        self.last_flush = Instant::now();
    }
}
