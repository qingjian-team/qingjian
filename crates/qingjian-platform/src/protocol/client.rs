use serde::{Deserialize, Serialize};

use super::indicator::IndicatorCommand;
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

    /// 组句期间 DLL 定时轮询：取云联想的异步结果（云端候选 / 整句补全），以及候选点击等非按键上屏。
    /// Server 拉一次 `poll_prediction`，把最新组句状态经 [`super::ServerMessage::Update`] 回给 DLL（`commit`
    /// 有则让 DLL 写入文档）。传输仍是一问一答，云结果靠 DLL 侧定时器拉取，不需要 Server 主动推。
    Poll {
        /// 会话标识。
        session: SessionId,
    },

    /// 组句起始时 DLL 主动送来的应用光标前文，给本地整句模型当前文（对应 macOS 壳在组句第一键读 `surrounding_text`）。
    /// 在起组句的那次编辑会话里顺手读，不另开会话、不回话；密码框 / 读不到时不发，Server 退回本会话历史。
    Surrounding {
        /// 会话标识。
        session: SessionId,

        /// 光标前最多 64 字。
        text: String,
    },

    /// 输入框私密与否变了（DLL 起组句时按输入范围判：`IS_PRIVATE` / 密码 / PIN 类算私密，浏览器无痕窗口就是它）。
    /// Server 让 Engine 进 / 出私密：不学习、不记输入日志、不发云端；前文 DLL 侧就不读。只在与上次报的不同时发，不回话。
    /// 真正的密码框（`KEYBOARD_DISABLED` compartment）DLL 直接放行所有键、不组句，到不了这里。
    Privacy {
        /// 会话标识。
        session: SessionId,

        /// 现在是私密输入。
        private: bool,
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

    /// 用户在这个应用里切了中英模式：DLL 报给 Server，成为全局模式（其余应用下次取模式时跟上，悬浮状态条同步）。
    /// 不等回话。双拼方案 Server 从自己的配置里知道，不必带。
    ModeChanged {
        /// 会话标识。
        session: SessionId,

        /// `true` 英文模式，`false` 中文模式。
        english: bool,
    },

    /// 取全局中英模式：激活、得到焦点时各一次，前台、没在组句时再定时问（别的应用或悬浮状态条可能切过）。
    /// 回 [`super::ServerMessage::ModeSync`]；有 DLL 来取也说明青简是当前输入法。
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

    /// 任务栏「中 / 英」图标右键菜单里点了一项（v7 起）。不等回话。
    Indicator {
        /// 会话标识。
        session: SessionId,

        /// 点的是哪一项。
        command: IndicatorCommand,
    },

    /// 关闭会话，释放 Server 侧状态。
    CloseSession {
        /// 会话标识。
        session: SessionId,
    },
}
