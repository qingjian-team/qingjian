//! 单个 Fcitx 输入上下文保存的输入状态。
use crate::dispatch::composed::Composed;
use qingjian_core::EngineSession;

pub(crate) struct SessionInfo {
    /// 应用标识。
    pub(crate) app: Option<String>,

    /// 新 Linux 客户端已协商显示回报。
    pub(crate) display_identity: Option<crate::protocol::DisplayIdentity>,

    /// 待确认的当前候选帧。
    pub(crate) display_frame: Option<qingjian_platform::protocol::Frame>,

    /// Server 持有的中英模式与单击 Shift 状态。
    pub(crate) english: bool,

    pub(crate) shift_pending: bool,

    /// 最近的框架能力，首次报告前为未知。
    pub(crate) capabilities: Option<crate::protocol::Capabilities>,

    pub(crate) disabled: bool,

    /// 焦点事实；只有当前聚焦帧允许展示回报。
    pub(crate) active: bool,

    /// 新会话默认私密，收到能力通知后才允许学习。
    pub(crate) private: bool,

    /// 挂起的 Engine 输入状态。
    pub(crate) engine: EngineSession,

    /// 挂起的候选列表。
    pub(crate) composed: Option<Composed>,

    /// 高亮下标。
    pub(crate) highlight: usize,

    /// 本轮是否移动候选。
    pub(crate) navigated: bool,
}
impl SessionInfo {
    pub(crate) fn new(app: Option<String>) -> Self {
        Self {
            app,
            english: false,
            shift_pending: false,
            capabilities: None,
            disabled: false,
            active: true,
            display_identity: None,
            display_frame: None,
            private: true,
            engine: EngineSession::default(),
            composed: None,
            highlight: 0,
            navigated: false,
        }
    }
}
