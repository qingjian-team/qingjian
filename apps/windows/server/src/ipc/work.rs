use std::sync::mpsc::Sender;

use qingjian_platform::protocol::{ClientMessage, ServerMessage};

use crate::dispatch::{CandidateMouse, PanelMouse, StatusEvent};

/// 工人线程（独占 [`crate::dispatch::Router`]）的一件活：DLL 的一条消息，或状态条 / 候选窗口 / 符号面板上的一次点击。
pub enum Work {
    /// 某条连接收到的消息 + 回结果的通道（`None` 表示不用回话）。
    Client(ClientMessage, Sender<Option<ServerMessage>>),

    /// UI 线程发来的状态条操作。
    Status(StatusEvent),

    /// UI 线程发来的候选窗口鼠标操作（点击、右键、菜单里点中的项）。
    Click(CandidateMouse),

    /// UI 线程发来的符号面板上的鼠标操作。
    Panel(PanelMouse),
}
