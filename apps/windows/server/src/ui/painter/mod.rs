//! 青简渲染器在 Windows 壳里的落地：字体库 + 渲染器一份，候选窗口与状态条共用（字形缓存共享）。
//! 配置 `[general] renderer = "system"` 时没有它，两个窗口走原来的 GDI 画法（过渡期退路）。

use std::cell::RefCell;
use std::rc::Rc;

use qingjian_platform::{CandidateRenderer, LayoutMode};
use qingjian_render::{
    FontLibrary, Frame, Layout, Mode, Rendered, RenderedStatus, Renderer, StatusCell, TextSizes,
    Theme, ThemeLibrary, UiFont, system_fonts,
};

use crate::dispatch::RenderSettings;

/// UI 线程上共享的渲染器；`None` = 系统绘制。
pub(super) type SharedPainter = Rc<RefCell<Option<Painter>>>;

pub(super) struct Painter {
    /// 渲染器（字体库随它）。
    renderer: Renderer,

    /// 建它时用的字族名（空为系统字体），设置没变就不重建。
    font: String,

    /// 主题（浅色那一份，画的时候按外观取深浅）。
    theme: Theme,
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
            theme: Theme::light(),
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
                // 设置变了才会走到这里（含主题文件的戳），每次都从主题目录重读
                if let Some(painter) = painter.as_mut() {
                    let themes =
                        ThemeLibrary::load(qingjian_platform::dirs::themes_dir().as_deref());
                    let (candidate, annotation) = settings.font_sizes;
                    painter.theme = themes
                        .resolve(&settings.theme, false)
                        .with_text_sizes(TextSizes {
                            candidate: candidate.get(),
                            annotation: annotation.get(),
                        })
                        .with_animations(settings.animations);
                    painter
                        .renderer
                        .load_theme_fonts(&painter.theme, system_fonts::family_files);
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

    /// 画一帧候选窗口；`dpi` 96 为 100%。失败记日志返回 `None`，调用方退回 GDI。
    pub(super) fn render_frame(
        &mut self,
        frame: &Frame,
        layout: LayoutMode,
        dark: bool,
        dpi: u32,
    ) -> Option<Rendered> {
        let layout = match layout {
            LayoutMode::Vertical => Layout::Vertical,
            LayoutMode::Horizontal => Layout::Horizontal,
        };
        let started = std::time::Instant::now();
        self.renderer.set_reduce_motion(reduce_motion());
        let rendered = self
            .renderer
            .render(frame, layout, &self.theme.with_dark(dark), scale(dpi))
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

    /// 动画的下一帧；没有在播的返回 `None`。
    pub(super) fn tick(&mut self) -> Option<Rendered> {
        self.renderer
            .tick()
            .inspect_err(|error| tracing::warn!(%error, "候选窗动画帧渲染失败"))
            .ok()
            .flatten()
    }

    /// 候选窗收起：忘掉上一帧，下次显示不从旧位置过渡。
    pub(super) fn forget(&mut self) {
        self.renderer.forget();
    }

    /// 画状态条。
    pub(super) fn render_status(
        &mut self,
        cells: &[StatusCell],
        mode: &Mode,
        dark: bool,
        dpi: u32,
    ) -> Option<RenderedStatus> {
        self.renderer
            .render_status(cells, mode, &self.theme.with_dark(dark), scale(dpi))
            .inspect_err(|error| tracing::warn!(%error, "状态条渲染失败"))
            .ok()
    }
}

/// 系统关了「动画效果」（设置 → 辅助功能 → 视觉效果）：不播过渡。读不到当开着。
fn reduce_motion() -> bool {
    let mut enabled = windows::core::BOOL(1);
    // SAFETY: SPI_GETCLIENTAREAANIMATION 往 pvParam 写一个 BOOL。
    let ok = unsafe {
        windows::Win32::UI::WindowsAndMessaging::SystemParametersInfoW(
            windows::Win32::UI::WindowsAndMessaging::SPI_GETCLIENTAREAANIMATION,
            0,
            Some((&raw mut enabled).cast()),
            windows::Win32::UI::WindowsAndMessaging::SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    ok.is_ok() && !enabled.as_bool()
}

/// 点 → 像素的倍数。
fn scale(dpi: u32) -> f32 {
    dpi.max(96) as f32 / 96.0
}
