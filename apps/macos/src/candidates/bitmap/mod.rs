//! 候选窗口的位图绘制：一帧交给 `qingjian-render` 画成位图，`drawRect:` 里贴上去。
//!
//! 与自绘 NSView 的旧路径并存：配置 `[general] renderer = "system"` 走旧路（过渡期退路）。
//! 阴影由主题定义、渲染器画进位图（位图四周留边），面板关掉系统阴影，与 Windows 一致。

mod convert;
mod font_files;

pub(crate) use font_files::{available_families, family_files};

use objc2::AnyThread;
use objc2::rc::Retained;
use objc2_app_kit::{NSBitmapImageRep, NSCalibratedRGBColorSpace, NSCompositingOperation, NSImage};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use qingjian_platform::LayoutMode;
use qingjian_render::{FontLibrary, HitRegion, HitTarget, Layout, Renderer, Theme, UiFont};

use super::bounds::ViewBounds;
use super::frame::Frame;

pub struct BitmapPainter {
    /// 渲染器（字体库随它）。
    renderer: Renderer,

    /// 最近一帧的位图，`None` 表示还没画过。
    image: Option<Retained<NSImage>>,

    /// 最近一帧（外观变了要重画）。
    frame: qingjian_render::Frame,

    /// 最近一帧的排布。
    layout: Layout,

    /// 最近一帧按深色画的。
    dark: bool,

    /// 最近一帧的倍数。
    scale: f32,

    /// 最近一帧的尺寸（点）与其中的内容区。
    bounds: ViewBounds,

    /// 主题（浅色那一份，画的时候按外观取深浅）。
    theme: Theme,

    /// 最近一帧还有动画在播：隔多久要调 [`Self::tick`]。
    next_frame: Option<std::time::Duration>,

    /// 最近一帧里候选与译词的点击区域（内容区像素）。
    hits: Vec<HitRegion>,

    /// 内容区左上角在视图里的位置（点，左上为原点）：位图四周留了投影的边。
    content_origin: NSPoint,
}

impl BitmapPainter {
    /// `font` 是用户选的字族名，空为系统字体；没装就回到系统字体。字体库加载失败返回 `None`，调用方退回旧路径。
    pub fn new(font: &str) -> Option<Self> {
        let started = std::time::Instant::now();
        let font = font.trim();
        let library = if font.is_empty() {
            FontLibrary::system("zh-CN")
        } else {
            let ui_font = UiFont {
                family: font.to_owned(),
                files: font_files::family_files(font),
            };
            FontLibrary::with_ui_font("zh-CN", &ui_font)
        };
        let library = match library {
            Ok(library) => library,
            Err(error) => {
                tracing::warn!(%error, "渲染器字体库加载失败，候选窗退回 AppKit 绘制");
                return None;
            }
        };
        tracing::info!(
            elapsed = ?started.elapsed(),
            font = library.ui_family(),
            "候选窗使用位图渲染器"
        );
        Some(Self {
            renderer: Renderer::new(library),
            image: None,
            frame: qingjian_render::Frame::default(),
            layout: Layout::Vertical,
            dark: false,
            scale: 2.0,
            bounds: ViewBounds::filled(NSSize::ZERO),
            theme: Theme::light(),
            next_frame: None,
            hits: Vec::new(),
            content_origin: NSPoint::ZERO,
        })
    }

    /// 换主题（浅色那一份）：加载它要的字体，用最近一帧当场重画。
    pub fn set_theme(&mut self, theme: Theme) {
        self.renderer
            .load_theme_fonts(&theme, font_files::family_files);
        self.theme = theme;
        self.repaint();
    }

    /// 记下新一帧并画好，返回视图该有的尺寸与内容区（点）。
    pub fn set_frame(
        &mut self,
        frame: &Frame,
        layout: LayoutMode,
        dark: bool,
        scale: f32,
    ) -> ViewBounds {
        self.frame = convert::frame(frame);
        self.renderer.set_reduce_motion(reduce_motion());
        self.layout = match layout {
            LayoutMode::Vertical => Layout::Vertical,
            LayoutMode::Horizontal => Layout::Horizontal,
        };
        self.dark = dark;
        self.scale = scale;
        self.repaint();
        self.bounds
    }

    /// 外观或倍数变了就重画一遍再贴。
    pub fn draw(&mut self, dark: bool, scale: f32) {
        if dark != self.dark || scale != self.scale {
            self.dark = dark;
            self.scale = scale;
            self.repaint();
        }
        let Some(image) = &self.image else {
            return;
        };
        let rect = NSRect::new(NSPoint::ZERO, self.bounds.size);
        // SAFETY: hints 传 None，其余参数都是普通值；在 drawRect: 内调用，有当前图形上下文。
        unsafe {
            image.drawInRect_fromRect_operation_fraction_respectFlipped_hints(
                rect,
                NSRect::ZERO,
                NSCompositingOperation::Copy,
                1.0,
                true,
                None,
            );
        }
    }

    /// 视图里一点（点，左上为原点）点中了哪个候选或哪条译词。
    pub fn hit(&self, point: NSPoint) -> Option<HitTarget> {
        let scale = f64::from(self.scale);
        let x = ((point.x - self.content_origin.x) * scale) as f32;
        let y = ((point.y - self.content_origin.y) * scale) as f32;
        self.hits
            .iter()
            .find(|region| region.contains(x, y))
            .map(|region| region.target)
    }

    /// 有动画在播时隔多久要下一帧。
    pub fn next_frame(&self) -> Option<std::time::Duration> {
        self.next_frame
    }

    /// 动画的下一帧：换上新位图（尺寸不变）；返回隔多久再要，`None` 为播完。
    pub fn tick(&mut self) -> Option<std::time::Duration> {
        match self.renderer.tick() {
            Ok(Some(rendered)) => {
                self.next_frame = rendered.next_frame;
                self.image = to_image(&rendered.pixmap, self.bounds.size);
            }
            Ok(None) => self.next_frame = None,
            Err(error) => {
                tracing::warn!(%error, "候选窗动画帧渲染失败");
                self.next_frame = None;
            }
        }
        self.next_frame
    }

    /// 窗口收起：停动画、忘掉上一帧，下次显示不从旧位置过渡。
    pub fn forget(&mut self) {
        self.renderer.forget();
        self.next_frame = None;
    }

    fn repaint(&mut self) {
        let theme = self.theme.with_dark(self.dark);
        let started = std::time::Instant::now();
        let rendered = match self
            .renderer
            .render(&self.frame, self.layout, &theme, self.scale)
        {
            Ok(rendered) => rendered,
            Err(error) => {
                tracing::warn!(%error, "候选窗渲染失败");
                self.image = None;
                self.hits.clear();
                return;
            }
        };
        let (width, height) = rendered.content_size_points();
        self.bounds = bounds(&rendered);
        self.next_frame = rendered.next_frame;
        self.content_origin = NSPoint::new(
            f64::from(rendered.content_x) / f64::from(rendered.scale),
            f64::from(rendered.content_y) / f64::from(rendered.scale),
        );
        self.hits = rendered.hits;
        self.image = to_image(&rendered.pixmap, self.bounds.size);
        tracing::debug!(elapsed = ?started.elapsed(), width, height, "候选窗位图已画");
    }
}

/// 系统辅助功能里开了「减少动态效果」。
fn reduce_motion() -> bool {
    objc2_app_kit::NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
}

/// 位图与内容区换成点；内容区原点换成左下角起算。
fn bounds(rendered: &qingjian_render::Rendered) -> ViewBounds {
    let points = |pixels: u32| f64::from(pixels) / f64::from(rendered.scale);
    let (width, height) = (rendered.pixmap.width(), rendered.pixmap.height());
    let below = height.saturating_sub(rendered.content_y + rendered.content_height);
    ViewBounds {
        size: NSSize::new(points(width), points(height)),
        content: NSRect::new(
            NSPoint::new(points(rendered.content_x), points(below)),
            NSSize::new(
                points(rendered.content_width),
                points(rendered.content_height),
            ),
        ),
    }
}

/// 预乘 RGBA 位图 → NSImage（尺寸按点，位图按像素，Retina 自然对上）。
fn to_image(pixmap: &qingjian_render::Pixmap, size: NSSize) -> Option<Retained<NSImage>> {
    let (width, height) = (pixmap.width(), pixmap.height());
    // SAFETY: planes 传空让 AppKit 自己分配；参数描述的是 8 位 × 4 通道、预乘 alpha 在后的连续 RGBA，
    // 与 tiny-skia 的内存布局一致；随后按 bytesPerRow 逐行拷进去，不越界。
    let rep = unsafe {
        let rep = NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            width as isize,
            height as isize,
            8,
            4,
            true,
            false,
            NSCalibratedRGBColorSpace,
            (width * 4) as isize,
            32,
        )?;
        let stride = rep.bytesPerRow() as usize;
        let data = rep.bitmapData();
        let row_bytes = width as usize * 4;
        for (row, source) in pixmap.data().chunks_exact(row_bytes).enumerate() {
            std::ptr::copy_nonoverlapping(source.as_ptr(), data.add(row * stride), row_bytes);
        }
        rep
    };
    let image = NSImage::initWithSize(NSImage::alloc(), size);
    image.addRepresentation(&rep);
    Some(image)
}
