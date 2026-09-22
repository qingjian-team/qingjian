//! Linux 会话路由：独立保存各上下文的组句，词库和学习服务保持单实例。本地整句模型在 [`rescore`]。

mod composed;
mod config;
mod display;
mod key;
mod linux;
mod message;
mod rescore;
mod session;

use self::composed::Composed;
pub use self::config::RouterConfig;
pub use self::rescore::find_model;
use self::rescore::{ModelLoader, RescoreState};
use self::session::SessionInfo;
use qingjian_core::Engine;
use qingjian_platform::protocol::{ClientMessage, ServerMessage, SessionId};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// 学习数据落盘间隔（与 macOS 壳一致）；Server 没有定时器，借消息节拍与主循环的 tick 看时间。
const LEARNING_FLUSH_INTERVAL: Duration = Duration::from_secs(60);

/// Fcitx5 输入上下文分派器；所有 Engine 操作都在 Server 主线程串行执行。
pub struct Router {
    /// 唯一的词库及学习服务，输入状态在会话切换时交换。
    engine: Engine,

    /// 按键及候选展示配置。
    config: RouterConfig,

    /// 会话表。
    sessions: HashMap<SessionId, SessionInfo>,

    /// 当前装入 Engine 的会话。
    focused: Option<SessionId>,

    /// 当前候选与 preedit。
    composed: Option<Composed>,

    /// 当前高亮的跨页下标。
    highlight: usize,

    /// 本轮是否已主动移动候选。
    navigated: bool,

    /// 可选的整句提示；首版没有云服务。
    sentence: Option<String>,

    /// 删除候选等操作提示。
    notice: Option<String>,

    /// 全服务帧号递增，关闭再开不会复用展示身份。
    display_revision: u64,

    /// 最近一次学习落盘的时刻。
    last_flush: Instant,

    /// 本地整句模型（`.qjm` 或三件套目录）；没有模型文件为 `None`。
    model_path: Option<PathBuf>,

    /// 进行中的模型加载；加载完接到 Engine 上就清掉。
    model_loader: Option<ModelLoader>,

    /// 重排的防抖 / 轮询进行态。
    rescore: RescoreState,
}

impl Router {
    pub fn new(engine: Engine, mut config: RouterConfig) -> Self {
        config.page_size = config.page_size.clamp(1, 9);
        config.cloud_slots = 0;
        Self {
            engine,
            config,
            sessions: HashMap::new(),
            focused: None,
            composed: None,
            highlight: 0,
            navigated: false,
            sentence: None,
            notice: None,
            display_revision: 0,
            last_flush: Instant::now(),
            model_path: None,
            model_loader: None,
            rescore: RescoreState::default(),
        }
    }
    /// 一条客户端消息；无需答复的通知返回 None。
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
    /// 到点了：接上加载好的模型、推进重排、到点落盘学习（输入停止后也不能一直不落盘）。主循环超时与插件的 `Poll` 都会调。
    pub fn tick(&mut self) {
        self.attach_loaded_model();
        self.advance_rescoring();
        if self.last_flush.elapsed() >= LEARNING_FLUSH_INTERVAL {
            self.flush_learning();
        }
    }
    pub(super) fn full_width_for(&self, english: bool) -> bool {
        if english {
            self.config.english_full_width
        } else {
            self.config.full_width
        }
    }
}

impl Drop for Router {
    fn drop(&mut self) {
        let sessions = self.sessions.keys().copied().collect::<Vec<_>>();
        for session in sessions {
            self.close_session(session);
        }
        self.flush_learning();
    }
}
