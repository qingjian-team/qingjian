//! 候选窗口的输出端。

use qingjian_platform::protocol::{Frame, ScreenRect};
use qingjian_platform::{CandidateRenderer, FontSize};
use qingjian_render::{HitTarget, Mode};

use crate::dispatch::{KeyHintView, PanelFrame};

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
    /// `key_hint` 是候选窗旁要不要贴键位提示，`None` 收起。
    fn show(&self, frame: Frame, mode: Mode, rect: ScreenRect, key_hint: Option<KeyHintView>);

    fn hide(&self);

    /// 画符号面板：贴在 `rect` 下方；没有矩形（从菜单打开、没在组句）时摆在鼠标旁，已显示着就留在原处。
    fn show_panel(&self, frame: PanelFrame, rect: Option<ScreenRect>);

    fn hide_panel(&self);

    /// 换画法：装上时与配置热加载后调，只在设置变了时调。
    fn configure(&self, settings: RenderSettings);

    /// 在候选窗口上弹右键菜单，点中的项经工人通道回给 Router。
    fn show_menu(&self, _menu: CandidateMenu) {}

    /// 弹出系统原生译文编辑框，确认后的文本经 [`CandidateMouse::TranslationEdited`] 回来。
    fn edit_translation(&self, _offset: usize, _current: String) {}

    /// 收起还弹着的右键菜单（敲了键、候选窗收起）。
    fn close_menu(&self) {}
}

/// 候选窗口上的鼠标操作（UI 线程经工人通道送给 Router）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CandidateMouse {
    /// 左键点中候选或译词。
    Click(HitTarget),

    /// 右键这一页第 `offset` 格，光标在屏幕 `point`（物理像素）。
    Menu { offset: usize, point: (i32, i32) },

    /// 右键菜单里点中了编号 `id` 的项。
    MenuPick { offset: usize, id: u32 },

    /// 原生编辑框确认了新的候选文本。
    TranslationEdited { offset: usize, text: String },
    /// 滚轮翻页：正数往后翻（点翻页箭头走 `Click(HitTarget::Page)`）。
    Page(isize),
}

/// 右键候选弹的菜单。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateMenu {
    /// 这一页第几格（从 0 起），点中的项连同它回给 Router。
    pub offset: usize,

    /// 弹在屏幕哪一点（物理像素）。
    pub point: (i32, i32),

    /// 各项的编号与标题（[`qingjian_platform::candidate_menu::CandidateAction`]）。
    pub items: Vec<(u32, &'static str)>,
}

/// 不画候选窗口的空实现。
pub struct NoopSink;

impl CandidateSink for NoopSink {
    fn show(&self, _frame: Frame, _mode: Mode, _rect: ScreenRect, _key_hint: Option<KeyHintView>) {}

    fn hide(&self) {}

    fn show_panel(&self, _frame: PanelFrame, _rect: Option<ScreenRect>) {}

    fn hide_panel(&self) {}

    fn configure(&self, _settings: RenderSettings) {}
}
