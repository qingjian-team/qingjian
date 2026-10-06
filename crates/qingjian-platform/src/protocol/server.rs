use serde::{Deserialize, Serialize};

use super::frame::Frame;
use super::indicator::IndicatorState;
use super::key::KeyOutcome;
use super::session::SessionId;
use crate::config::SwitchKeys;

/// Server 下发给 DLL 的「按键行为」设置。
///
/// DLL 跑在每个应用的进程里，拿不到 Server 那份 [`Config`](crate::Config)，但这两个值在**按键到达之前**
/// 就得知道：单击切换键的判定在 `OnTestKeyUp` 里做，内置英文模式开关决定要不要登记语言栏按钮。
/// 所以由 Server 读配置（它本来就在盯热加载）经协议下发，DLL 不读文件、不查 mtime——
/// `%APPDATA%\Qingjian` 对 AppContainer 里的商店应用本来也读不到。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputSettings {
    /// 中英切换键（`[shortcut] switch_mode`）。
    pub switch_mode: SwitchKeys,

    /// 内置英文模式总开关（`[general] english_mode`）。
    pub english_mode: bool,

    /// 中文模式下 Shift+字母进组句（`[general] shift_letter = "compose"`）。DLL 据此决定没在组句时
    /// 按住 Shift 敲的字母吃不吃：缺省交给应用，开着时送 Server 起一段组句（`⇧C` 接 `pan` 出「C盘」）。
    #[serde(default)]
    pub shift_letter_compose: bool,
}

impl Default for InputSettings {
    fn default() -> Self {
        Self {
            switch_mode: SwitchKeys::default(),
            english_mode: true,
            shift_letter_compose: false,
        }
    }
}

/// Server 发给 DLL 的消息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerMessage {
    /// 对一次 [`super::ClientMessage::OpenSession`] 的答复：把 DLL 在按键到达之前就要知道的
    /// 设置带过去一次（之后 [`Self::ModeSync`] 的每一拍也带着，改了配置不用重开会话）。
    ///
    /// **只回过协议版本对得上的 DLL**：老的 `open` 是只写不读，多回一条会被它当成下一次
    /// `Poll` 的应答而报错（见 Server 侧 `handle`）。
    SessionOpened {
        /// 会话标识。
        session: SessionId,

        /// 按键行为设置。
        input: InputSettings,
    },

    /// 对一次 [`super::ClientMessage::Key`] 的处理结果。
    KeyResult {
        /// 会话标识。
        session: SessionId,

        /// 这次按键吃掉还是放行。
        outcome: KeyOutcome,

        /// 本次要立即上屏的文本（选词 / 空格上屏 / 标点等）；没有则为 `None`。
        commit: Option<String>,

        /// 处理后要绘制的组句状态（preedit + 候选）；空 [`Frame`] 表示收起候选窗口。
        frame: Frame,
    },

    /// 对一次 [`super::ClientMessage::Commit`] 的答复：缓冲区里原样上屏的文本（拼音字母 / 英文模式下敲的字母）；
    /// 没在组句时为 `None`。Server 侧组句已清空，DLL 收到后把文本落进文档并收起组句。
    Committed {
        /// 会话标识。
        session: SessionId,

        /// 要原样上屏的文本。
        text: Option<String>,
    },

    /// 不由按键触发的重绘（云联想补词、本地整句模型重排到达）。
    Update {
        /// 会话标识。
        session: SessionId,

        /// 要重绘的状态。
        frame: Frame,
    },

    /// 对一次 [`super::ClientMessage::SyncMode`] 的答复：当前的全局中英模式，
    /// 外加当前的按键行为设置（每一拍都带，DLL 那边热加载就靠它）。
    ModeSync {
        /// 会话标识。
        session: SessionId,

        /// 全局模式：`Some(true)` 英文、`Some(false)` 中文，DLL 与自己不同就跟上；`None` 不动（老 Server 没有待切换时）。
        english: Option<bool>,

        /// 按键行为设置；老 DLL 不认识这个字段，读到时忽略（serde 默认忽略多余字段）。
        #[serde(default)]
        input: InputSettings,

        /// 右键菜单打勾用的开关状态（v7 起）。
        #[serde(default)]
        indicator: IndicatorState,
    },

    /// 收到「翻译选中文字」快捷键：请 DLL 在读编辑会话里取当前选区，用
    /// [`super::ClientMessage::Selection`] 回。这是对触发快捷键那次 [`super::ClientMessage::Key`] 的应答
    /// （替代常规 [`Self::KeyResult`]）；随后 DLL 发来的 `Selection` 才引出翻译候选帧。
    RequestSelection {
        /// 会话标识。
        session: SessionId,

        /// 请求标识，回时带上。
        request: u64,
    },
}
