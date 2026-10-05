//! 青简渲染器在 Windows 壳里的落地：字体库 + 渲染器一份，候选窗口与状态条共用（字形缓存共享）。
//! 配置 `[general] renderer = "system"` 时没有它，两个窗口走原来的 GDI 画法（过渡期退路）。

use std::cell::RefCell;
use std::rc::Rc;

use qingjian_platform::{CandidateRenderer, LayoutMode};
use qingjian_render::{
    FontLibrary, Frame, Layout, Rendered, RenderedStatus, Renderer, Shadow, StatusCell, Theme,
    UiFont, system_fonts,
};

use crate::dispatch::RenderSettings;

/// UI 线程上共享的渲染器；`None` = 系统绘制。
pub(super) type SharedPainter = Rc<RefCell<Option<Painter>>>;

pub(super) struct Painter {
    /// 渲染器（字体库随它）。
    renderer: Renderer,

    /// 建它时用的字族名（空为系统字体），设置没变就不重建。
    font: String,
}

impl Painter {
    /// `font` 是用户选的字族名，空为系统字体；没装就回到系统字体。字体库加载失败返回 `None`，调用方退回 GDI。
    fn new(font: &str) -> Option<Self> {
        let started = std::time::Instant::now();
        let library = if font.is_empty() {
            FontLibrary::system("zh-CN")
        } else {
            let ui_font = UiFont {
                family: font.to_owned(),
                files: system_fonts::family_files(font),
            };
            FontLibrary::with_ui_font("zh-CN", &ui_font)
        };
        let library = match library {
            Ok(library) => library,
            Err(error) => {
                tracing::warn!(%error, "渲染器字体库加载失败，候选窗口与状态条退回 GDI 绘制");
                return None;
            }
        };
        tracing::info!(
            elapsed = ?started.elapsed(),
            font = library.ui_family(),
            "候选窗口与状态条使用青简渲染器"
        );
        Some(Self {
            renderer: Renderer::new(library),
            font: font.to_owned(),
        })
    }

    /// 按设置建 / 换 / 撤渲染器。
    pub(super) fn configure(shared: &SharedPainter, settings: &RenderSettings) {
        let mut painter = shared.borrow_mut();
        match settings.renderer {
            CandidateRenderer::Qingjian => {
                if painter.as_ref().map(|p| p.font.as_str()) != Some(settings.font.as_str()) {
                    *painter = Self::new(&settings.font);
                }
            }
            CandidateRenderer::System => {
                if painter.is_some() {
                    tracing::info!("候选窗口与状态条切回 GDI 绘制");
                    *painter = None;
                }
            }
        }
    }

    /// 画一帧候选窗口；`dpi` 96 为 100%，`font_size` 是候选词字号（点，译文与序号等比跟着缩放）。
    /// 失败记日志返回 `None`，调用方退回 GDI。
    pub(super) fn render_frame(
        &mut self,
        frame: &Frame,
        layout: LayoutMode,
        dark: bool,
        dpi: u32,
        font_size: f32,
    ) -> Option<Rendered> {
        let layout = match layout {
            LayoutMode::Vertical => Layout::Vertical,
            LayoutMode::Horizontal => Layout::Horizontal,
        };
        let started = std::time::Instant::now();
        let rendered = self
            .renderer
            .render(
                frame,
                layout,
                &candidate_theme(dark, font_size),
                scale(dpi),
                Some(&SHADOW),
            )
            .inspect_err(|error| tracing::warn!(%error, "候选窗渲染失败"))
            .ok()?;
        tracing::debug!(
            elapsed = ?started.elapsed(),
            width = rendered.content_width,
            height = rendered.content_height,
            "候选窗位图已画"
        );
        Some(rendered)
    }

    /// 画状态条。
    pub(super) fn render_status(
        &mut self,
        cells: &[StatusCell],
        dark: bool,
        dpi: u32,
    ) -> Option<RenderedStatus> {
        self.renderer
            .render_status(cells, &theme(dark), scale(dpi), Some(&SHADOW))
            .inspect_err(|error| tracing::warn!(%error, "状态条渲染失败"))
            .ok()
    }
}

/// 两个窗口都用渲染器画阴影（分层窗口没有系统阴影），参数与 macOS 面板一致。
const SHADOW: Shadow = Shadow::mac_panel();

fn theme(dark: bool) -> Theme {
    if dark { Theme::dark() } else { Theme::light() }
}

/// 候选窗主题：按用户字号缩放三种字体（译文与序号等比跟着）；状态条用 [`theme`] 不缩放。
fn candidate_theme(dark: bool, font_size: f32) -> Theme {
    theme(dark).with_font_size(font_size)
}

/// 点 → 像素的倍数。
fn scale(dpi: u32) -> f32 {
    dpi.max(96) as f32 / 96.0
}
