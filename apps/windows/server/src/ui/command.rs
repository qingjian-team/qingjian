//! 交给 UI 线程执行的命令。

use qingjian_platform::protocol::{Frame, ScreenRect};
use qingjian_render::Mode;

use crate::dispatch::{CandidateMenu, KeyHintView, PanelFrame, RenderSettings, StatusView};

/// 交给 UI 线程执行的命令。`Frame` 较大，装箱免得枚举过胖。
pub(super) enum UiCommand {
    /// 把候选窗口摆到组句矩形下方并按帧重绘；带着键位提示就在旁边贴上键盘。
    Show(Box<(Frame, Mode, ScreenRect, Option<KeyHintView>)>),

    /// 收起候选窗口。
    Hide,

    /// 显示 / 更新符号面板；没有光标矩形时摆在鼠标旁。
    PanelShow(Box<(PanelFrame, Option<ScreenRect>)>),

    /// 收起符号面板。
    PanelHide,

    /// 显示 / 更新悬浮状态条。
    StatusShow(Box<StatusView>),

    /// 收起悬浮状态条。
    StatusHide,

    /// 换画法（渲染器 / 字体）。
    Configure(RenderSettings),

    /// 在候选窗口上弹右键菜单。
    Menu(CandidateMenu),

    /// 收起右键菜单。
    MenuClose,
}
