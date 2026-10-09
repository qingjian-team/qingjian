//! 交给 UI 线程执行的命令。

use qingjian_platform::protocol::{Frame, ScreenRect};
use qingjian_render::Mode;

use crate::dispatch::{RenderSettings, StatusView};

/// 交给 UI 线程执行的命令。`Frame` 较大，装箱免得枚举过胖。
pub(super) enum UiCommand {
    /// 把候选窗口摆到组句矩形下方并按帧重绘。
    Show(Box<(Frame, Mode, ScreenRect)>),

    /// 收起候选窗口。
    Hide,

    /// 显示 / 更新悬浮状态条。
    StatusShow(Box<StatusView>),

    /// 收起悬浮状态条。
    StatusHide,

    /// 换画法（渲染器 / 字体）。
    Configure(RenderSettings),
}
