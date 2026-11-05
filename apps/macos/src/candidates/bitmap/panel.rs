//! 符号面板的位图：面板一帧交给 `qingjian-render` 画成位图，`drawRect:` 里贴上去，点击区域随位图一起给出。
//! 候选窗选了 AppKit 绘制时面板也走这里（主题用青简绿），所以单独一份渲染器，不跟 [`super::BitmapPainter`] 走。

use objc2::rc::Retained;
use objc2_app_kit::{NSCompositingOperation, NSImage};
use objc2_foundation::{NSPoint, NSRect};
use qingjian_render::{FontLibrary, HitTarget, PanelFrame, PanelHit, Renderer, Theme};

use super::{bounds, font_files, to_image};
use crate::candidates::PanelMouse;
use crate::candidates::bounds::ViewBounds;

pub struct PanelPainter {
    renderer: Renderer,

    /// 最近一帧（外观或倍数变了要重画）。
    frame: PanelFrame,

    image: Option<Retained<NSImage>>,

    bounds: ViewBounds,

    /// 点击区域（内容区像素）。
    hits: Vec<qingjian_render::HitRegion>,

    /// 内容区左上角在视图里的位置（点，左上为原点）。
    content_origin: NSPoint,

    /// 主题（浅色那一份）。
    theme: Theme,

    dark: bool,

    scale: f32,
}

impl PanelPainter {
    /// 字体库加载失败返回 `None`，面板不显示。
    pub fn new(theme: Theme) -> Option<Self> {
        let library = match FontLibrary::system("zh-CN") {
            Ok(library) => library,
            Err(error) => {
                tracing::warn!(%error, "symbol panel font library failed to load");
                return None;
            }
        };
        let mut renderer = Renderer::new(library);
        renderer.load_theme_fonts(&theme, font_files::family_files);
        Some(Self {
            renderer,
            frame: PanelFrame::default(),
            image: None,
            bounds: ViewBounds::filled(objc2_foundation::NSSize::ZERO),
            hits: Vec::new(),
            content_origin: NSPoint::ZERO,
            theme,
            dark: false,
            scale: 2.0,
        })
    }

    /// 换主题：加载它要的字体，下一帧起用新的。
    pub fn set_theme(&mut self, theme: Theme) {
        self.renderer
            .load_theme_fonts(&theme, font_files::family_files);
        self.theme = theme;
    }

    /// 记下新一帧并画好，返回视图该有的尺寸与内容区（点）。
    pub fn set_frame(&mut self, frame: &PanelFrame, dark: bool, scale: f32) -> ViewBounds {
        self.frame = frame.clone();
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

    /// 视图里一点（点，左上为原点）点中了什么；空白处为 `None`。
    pub fn hit(&self, point: NSPoint, double: bool) -> Option<PanelMouse> {
        let scale = f64::from(self.scale);
        let x = ((point.x - self.content_origin.x) * scale) as f32;
        let y = ((point.y - self.content_origin.y) * scale) as f32;
        let target = self
            .hits
            .iter()
            .find(|region| region.contains(x, y))?
            .target;
        match target {
            HitTarget::Panel(PanelHit::Cell(offset)) => Some(PanelMouse::Cell { offset, double }),
            HitTarget::Panel(PanelHit::Group(group)) => Some(PanelMouse::Group(group)),
            HitTarget::Panel(PanelHit::Section(section)) => Some(PanelMouse::Section(section)),
            HitTarget::Panel(PanelHit::Candidate(index)) => Some(PanelMouse::Candidate(index)),
            HitTarget::Panel(PanelHit::Caret(position)) => Some(PanelMouse::Caret {
                position,
                extend: false,
            }),
            HitTarget::Panel(PanelHit::Close) => Some(PanelMouse::Close),
            _ => None,
        }
    }

    fn repaint(&mut self) {
        let theme = self.theme.with_dark(self.dark);
        let rendered = match self.renderer.render_panel(&self.frame, &theme, self.scale) {
            Ok(rendered) => rendered,
            Err(error) => {
                tracing::warn!(%error, "symbol panel render failed");
                self.image = None;
                self.hits.clear();
                return;
            }
        };
        self.bounds = bounds(&rendered);
        self.content_origin = NSPoint::new(
            f64::from(rendered.content_x) / f64::from(rendered.scale),
            f64::from(rendered.content_y) / f64::from(rendered.scale),
        );
        self.image = to_image(&rendered.pixmap, self.bounds.size);
        self.hits = rendered.hits;
    }
}
