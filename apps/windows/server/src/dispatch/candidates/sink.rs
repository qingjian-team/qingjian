//! 候选窗口的输出端。

use qingjian_platform::protocol::{Frame, ScreenRect};
use qingjian_platform::{CandidateRenderer, FontSize};
use qingjian_render::Mode;

use crate::dispatch::PanelFrame;

/// 候选窗口 / 状态条的画法。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSettings {
    /// 由谁画（`[general] renderer`）。
    pub renderer: CandidateRenderer,

    /// 字族名（`[general] font`），空为系统字体。
    pub font: String,

    /// 主题 id（`[general] theme`，已按旧写法兼容解析）。
    pub theme: String,

    /// 候选字、译文字号（`[general] candidate_font_size` / `annotation_font_size`），盖过主题。
    pub font_sizes: (FontSize, FontSize),

    /// 过渡动画（`[general] animations`）。
    pub animations: bool,

    /// 用户主题目录的戳（`ThemeLibrary::stamp`）：主题文件改了它就变，UI 线程据此重读主题。
    pub themes_stamp: u64,
}

/// Router 只产出帧，画交给它；Windows 上由 UI 线程实现。
pub trait CandidateSink: Send {
    /// 把候选窗口摆到 `rect`（组句范围的屏幕矩形）下方并按 `frame` 重绘；`mode` 给主题显示中 / 英等状态。
    fn show(&self, frame: Frame, mode: Mode, rect: ScreenRect);

    fn hide(&self);

    /// 画符号面板：贴在 `rect` 下方；没有矩形（从菜单打开、没在组句）时摆在鼠标旁，已显示着就留在原处。
    fn show_panel(&self, frame: PanelFrame, rect: Option<ScreenRect>);

    fn hide_panel(&self);

    /// 换画法：装上时与配置热加载后调，只在设置变了时调。
    fn configure(&self, settings: RenderSettings);
}

/// 不画候选窗口的空实现。
pub struct NoopSink;

impl CandidateSink for NoopSink {
    fn show(&self, _frame: Frame, _mode: Mode, _rect: ScreenRect) {}

    fn hide(&self) {}

    fn show_panel(&self, _frame: PanelFrame, _rect: Option<ScreenRect>) {}

    fn hide_panel(&self) {}

    fn configure(&self, _settings: RenderSettings) {}
}
