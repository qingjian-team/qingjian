use serde::{Deserialize, Serialize};

use super::key::KeyEvent;
use super::screen_rect::ScreenRect;
use super::session::SessionId;

/// DLL（客户端，每个应用进程里一个）发给 Server 的消息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientMessage {
    /// 进入一个 TSF 文档，开一个会话。
    OpenSession {
        /// 会话标识。
        session: SessionId,

        /// 宿主应用的 exe 文件名（`Code.exe`），DLL 加载在应用进程里直接取；取不到为 `None`。
        /// Server 据此查 `[apps]` 分节的按应用设置（对应 macOS 的 bundle identifier）。
        #[serde(default)]
        app: Option<String>,

        /// DLL 编译时的 [`super::PROTOCOL_VERSION`]。升级安装后旧 DLL 仍留在没重启的应用里，
        /// Server 对不上只记警告照常服务；老 DLL 不带此字段，读成 0。
        #[serde(default)]
        protocol: u32,
    },

    /// 一次按键，等 Server 回 [`super::ServerMessage::KeyResult`]。
    Key {
        /// 会话标识。
        session: SessionId,

        /// 按键内容。
        event: KeyEvent,
    },

    /// 焦点离开 / 文档要求结束组句：Server 把缓冲区里的内容原样交出并清空，回 [`super::ServerMessage::Committed`]。
    Commit {
        /// 会话标识。
        session: SessionId,
    },

    /// 组句期间 DLL 定时轮询：取云联想的异步结果（云端候选 / 整句补全）。Server 拉一次
    /// `poll_prediction`，把最新组句状态经 [`super::ServerMessage::Update`] 回给 DLL。传输仍是一问一答，
    /// 云结果靠 DLL 侧定时器拉取，不需要 Server 主动推。
    Poll {
        /// 会话标识。
        session: SessionId,
    },

    /// 回应 [`super::ServerMessage::RequestSurrounding`]：应用光标前的一段文本，供整句前文用。
    Surrounding {
        /// 会话标识。
        session: SessionId,

        /// 请求标识，对上是哪一次询问。
        request: u64,

        /// 光标前最多约 64 字的上下文；取不到时为空串。
        text: String,
    },

    /// 回应 [`super::ServerMessage::RequestSelection`]：应用当前选中的文字（供「翻译选中文字」）。
    /// DLL 在读编辑会话里用 `GetSelection` + `GetText` 取；没有选区 / 读不到时 `text` 为空串。
    Selection {
        /// 会话标识。
        session: SessionId,

        /// 请求标识，对上是哪一次 [`super::ServerMessage::RequestSelection`]。
        request: u64,

        /// 选中的文字；没有选区时为空串。
        text: String,

        /// 选区的屏幕矩形（拿翻译候选窗口摆在它下方，与组句候选窗一致）；取不到是鼠标处近似。
        rect: ScreenRect,
    },

    /// 组句更新后，DLL 在编辑会话里量到组句范围的屏幕矩形，发来让 Server 把候选窗口摆到光标下方。
    /// 不等回话：候选窗口由 Server 进程自绘（搬出应用进程，才能盖过微软商店 / 任务栏搜索这些高 z-band 宿主）。
    /// 组句结束 / 失焦时 Server 按空帧与 [`Commit`](Self::Commit) 自行收窗口，不必 DLL 再发。
    PositionCandidates {
        /// 会话标识。
        session: SessionId,

        /// 组句范围的屏幕矩形（拿不到时是鼠标处的一个近似矩形）。
        rect: ScreenRect,
    },

    /// 组句在 DLL 侧结束、而 Server 无从知晓时（应用强行终止组句 `OnCompositionTerminated`、断连兜底），
    /// 让 Server 收起候选窗口。Server 按空帧 / [`Commit`](Self::Commit) 能自行收窗口的场合不需要这条。
    /// 不等回话。
    HideCandidates {
        /// 会话标识。
        session: SessionId,
    },

    /// 中英模式变化 / 获得焦点：DLL 把当前会话的持久中英模式推给 Server（供悬浮状态条显示当前中 / 英）。
    /// 单击 Shift 切换、激活、获焦时都发一次；不等回话（模式只在 DLL 侧，Server 据此刷状态条、并当作
    /// 「这个会话此刻聚焦」）。双拼方案 Server 从自己的配置里知道，不必带。
    ModeChanged {
        /// 会话标识。
        session: SessionId,

        /// `true` 英文模式，`false` 中文模式。
        english: bool,
    },

    /// 前台、没在组句时 DLL 定时问一次：用户在悬浮状态条上点过「中 / 英」没有。中英模式只在 DLL 侧，
    /// Server 只能记下「想切成哪个」等 DLL 来取，回 [`super::ServerMessage::ModeSync`]。
    SyncMode {
        /// 会话标识。
        session: SessionId,
    },

    /// 宿主线程把输入法切成了别的（微软拼音等）：Server 收起悬浮状态条。应用退出时不发（那时状态条该留着），
    /// 所以状态条的显隐不跟会话开关走。DLL 在被停用后才收到这个通知，用一条临时连接发；不等回话。
    ImeSwitched {
        /// 会话标识。
        session: SessionId,
    },

    /// 关闭会话，释放 Server 侧状态。
    CloseSession {
        /// 会话标识。
        session: SessionId,
    },
}
