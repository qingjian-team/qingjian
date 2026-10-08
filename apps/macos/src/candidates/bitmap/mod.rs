//! 候选窗口的位图绘制：一帧交给 `qingjian-render` 画成位图，`drawRect:` 里贴上去。
//!
//! 与自绘 NSView 的旧路径并存：配置 `[general] renderer = "system"` 走旧路（过渡期退路）。
//! 面板背景透明、系统阴影按位图的 alpha 走，所以渲染器不画阴影。

mod convert;
mod font_files;
#[cfg(test)]
mod tests;

pub(crate) use font_files::available_families;

use objc2::AnyThread;
use objc2::rc::Retained;
use objc2_app_kit::{NSBitmapImageRep, NSCalibratedRGBColorSpace, NSCompositingOperation, NSImage};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use qingjian_platform::LayoutMode;
use qingjian_render::{FontLibrary, Layout, Renderer, Theme, UiFont};

use super::colors::CandidateColors;
use super::frame::Frame;
use super::typography::Typography;

pub struct BitmapPainter {
    /// 渲染器（字体库随它）。
    renderer: Renderer,

    typography: Typography,

    colors: CandidateColors,

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

    /// 最近一帧的尺寸（点）。
    size: NSSize,
}

impl BitmapPainter {
    /// 字体库加载失败返回 `None`，调用方退回 AppKit。
    pub fn new(typography: &Typography, colors: &CandidateColors) -> Option<Self> {
        let started = std::time::Instant::now();
        let candidate = UiFont {
            family: typography.candidate_family.clone(),
            files: font_files::family_files(&typography.candidate_family),
        };
        let annotation = UiFont {
            family: typography.annotation_family.clone(),
            files: font_files::family_files(&typography.annotation_family),
        };
        let library = FontLibrary::with_candidate_fonts("zh-CN", &candidate, &annotation);
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
            typography: typography.clone(),
            colors: colors.clone(),
            image: None,
            frame: qingjian_render::Frame::default(),
            layout: Layout::Vertical,
            dark: false,
            scale: 2.0,
            size: NSSize::ZERO,
        })
    }

    pub fn set_colors(&mut self, colors: &CandidateColors) {
        self.colors = colors.clone();
        if self.image.is_some() {
            self.repaint();
        }
    }

    /// 记下新一帧并画好，返回窗口该有的尺寸（点）。
    pub fn set_frame(
        &mut self,
        frame: &Frame,
        layout: LayoutMode,
        dark: bool,
        scale: f32,
    ) -> NSSize {
        self.frame = convert::frame(frame);
        self.layout = match layout {
            LayoutMode::Vertical => Layout::Vertical,
            LayoutMode::Horizontal => Layout::Horizontal,
        };
        self.dark = dark;
        self.scale = scale;
        self.repaint();
        self.size
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
        let rect = NSRect::new(NSPoint::ZERO, self.size);
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

    fn repaint(&mut self) {
        let mut theme = if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        };
        self.typography.apply(&mut theme);
        theme.colors = self.colors.palette(self.dark);
        let started = std::time::Instant::now();
        let rendered =
            match self
                .renderer
                .render(&self.frame, self.layout, &theme, self.scale, None)
            {
                Ok(rendered) => rendered,
                Err(error) => {
                    tracing::warn!(%error, "候选窗渲染失败");
                    self.image = None;
                    return;
                }
            };
        let (width, height) = rendered.content_size_points();
        self.size = NSSize::new(f64::from(width), f64::from(height));
        self.image = to_image(&rendered.pixmap, self.size);
        tracing::debug!(elapsed = ?started.elapsed(), width, height, "候选窗位图已画");
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
