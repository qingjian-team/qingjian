//! 系统材质：主题写了 `material` 时垫在候选位图底下的那层系统视图，大小与圆角跟着内容区走。
//!
//! 面板的内容视图是一个空容器，里面材质视图在下、候选视图在上（铺满容器）；阴影交给面板按窗口形状算。
//! 玻璃用 macOS 26 的 `NSGlassEffectView`，更早的系统没有这个类，退回毛玻璃 `NSVisualEffectView`。
//! 新玻璃会随背后内容变深变浅，并改它里面内容的外观。主题写了 `appearance: "auto"` 时玻璃里放一个不画东西的探针，
//! 候选视图按探针的深浅取主题，文字跟着反色；没写时要材质跟锁定的外观或系统外观走，可新玻璃不认设给它的外观
//! （macOS 26.6 实测，压在深色背景上照样变深），所以换成毛玻璃再钉外观。

use std::cell::RefCell;

use block2::RcBlock;
use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyClass, Bool};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSAutoresizingMaskOptions, NSBezierPath, NSColor, NSGlassEffectView, NSImage,
    NSImageResizingMode, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
    NSVisualEffectState, NSVisualEffectView, NSWindowOrderingMode,
};
use objc2_foundation::{NSArray, NSEdgeInsets, NSRect, NSSize};
use qingjian_render::{Material, MaterialBacking};

use super::view::CandidateView;

pub struct ProbeIvars {
    /// 外观变了叫它重画。
    candidate: RefCell<Weak<CandidateView>>,
}

define_class!(
    // SAFETY: NSView 允许子类化；没有实现 Drop。
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ProbeIvars]
    struct AppearanceProbe;

    impl AppearanceProbe {
        #[unsafe(method(viewDidChangeEffectiveAppearance))]
        fn appearance_changed(&self) {
            // SAFETY: 调父类的同名方法，无参数
            let _: () = unsafe { msg_send![super(self), viewDidChangeEffectiveAppearance] };
            let dark = self.effectiveAppearance().name().to_string().contains("Dark");
            tracing::debug!(dark, "玻璃随背后内容换了外观");
            if let Some(candidate) = self.ivars().candidate.borrow().load() {
                candidate.setNeedsDisplay(true);
            }
        }
    }
);

impl AppearanceProbe {
    fn new(mtm: MainThreadMarker, candidate: &CandidateView) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ProbeIvars {
            candidate: RefCell::new(Weak::from(candidate)),
        });
        // SAFETY: NSView 的指定初始化方法
        unsafe { msg_send![super(this), initWithFrame: NSRect::ZERO] }
    }
}

pub struct Backdrop {
    /// 面板的内容视图。
    container: Retained<NSView>,

    /// 候选视图：材质视图插在它下面。
    candidate: Retained<CandidateView>,

    /// 正垫着的材质视图，连同它的种类与圆角（点）。
    installed: Option<Installed>,
}

struct Installed {
    material: Material,

    radius: f64,

    /// 文字跟着材质反色时玻璃里的探针。
    probe: Option<Retained<AppearanceProbe>>,

    view: MaterialView,
}

enum MaterialView {
    Glass(Retained<NSGlassEffectView>),

    Blur(Retained<NSVisualEffectView>),
}

impl MaterialView {
    fn as_view(&self) -> &NSView {
        match self {
            Self::Glass(view) => view,
            Self::Blur(view) => view,
        }
    }
}

impl Backdrop {
    pub fn new(mtm: MainThreadMarker, candidate: &Retained<CandidateView>) -> Self {
        let container = NSView::initWithFrame(NSView::alloc(mtm), candidate.frame());
        candidate.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        container.addSubview(candidate);
        Self {
            container,
            candidate: candidate.clone(),
            installed: None,
        }
    }

    /// 装进面板的内容视图。
    pub fn container(&self) -> &NSView {
        &self.container
    }

    /// 按这一帧垫材质：`content` 是内容区（视图坐标，原点左下），圆角已换成点；`None` 撤掉。
    pub fn update(&mut self, material: Option<MaterialBacking>, content: NSRect) {
        let Some(mut backing) = material else {
            if let Some(installed) = self.installed.take() {
                installed.view.as_view().removeFromSuperview();
                self.candidate.set_appearance_source(None);
            }
            return;
        };
        if backing.kind == Material::Glass && !backing.follows_backdrop {
            backing.kind = Material::Blur;
        }
        let reusable = self
            .installed
            .as_ref()
            .is_some_and(|installed| installed.material == backing.kind);
        if !reusable {
            if let Some(installed) = self.installed.take() {
                installed.view.as_view().removeFromSuperview();
            }
            let view = build(self.container.mtm(), backing.kind);
            self.container.addSubview_positioned_relativeTo(
                view.as_view(),
                NSWindowOrderingMode::Below,
                Some(&self.candidate),
            );
            self.installed = Some(Installed {
                material: backing.kind,
                radius: -1.0,
                probe: None,
                view,
            });
        }
        let Some(installed) = &mut self.installed else {
            return;
        };
        let view = installed.view.as_view();
        view.setFrame(content);
        let radius = f64::from(backing.radius);
        if installed.radius != radius {
            installed.radius = radius;
            match &installed.view {
                MaterialView::Glass(view) => view.setCornerRadius(radius),
                MaterialView::Blur(view) => view.setMaskImage(Some(&rounded_mask(radius))),
            }
        }
        match (&installed.view, backing.follows_backdrop) {
            (MaterialView::Glass(glass), true) => {
                if installed.probe.is_none() {
                    let probe = AppearanceProbe::new(self.container.mtm(), &self.candidate);
                    glass.setContentView(Some(&probe));
                    self.candidate
                        .set_appearance_source(Some(Retained::into_super(probe.clone())));
                    installed.probe = Some(probe);
                }
                glass.setAppearance(None);
            }
            _ => {
                if installed.probe.take().is_some() {
                    if let MaterialView::Glass(glass) = &installed.view {
                        glass.setContentView(None);
                    }
                    self.candidate.set_appearance_source(None);
                }
                let dark = backing
                    .locked_dark
                    .unwrap_or_else(|| is_dark(&self.container.effectiveAppearance()));
                view.setAppearance(named(dark).as_deref());
            }
        }
    }
}

/// 外观是不是深色。
fn is_dark(appearance: &NSAppearance) -> bool {
    // SAFETY: 只读 AppKit 导出的常量名
    let names = unsafe { NSArray::from_slice(&[NSAppearanceNameAqua, NSAppearanceNameDarkAqua]) };
    let matched = appearance.bestMatchFromAppearancesWithNames(&names);
    // SAFETY: 同上
    matched.is_some_and(|name| unsafe { name.isEqualToString(NSAppearanceNameDarkAqua) })
}

/// 浅色 / 深色的系统外观。
fn named(dark: bool) -> Option<Retained<NSAppearance>> {
    // SAFETY: 只读 AppKit 导出的常量名
    let name = unsafe {
        if dark {
            NSAppearanceNameDarkAqua
        } else {
            NSAppearanceNameAqua
        }
    };
    NSAppearance::appearanceNamed(name)
}

/// 建材质视图：玻璃要系统有 `NSGlassEffectView`（macOS 26 起），没有就退回毛玻璃。
fn build(mtm: MainThreadMarker, material: Material) -> MaterialView {
    let frame = NSRect::ZERO;
    if material == Material::Glass && AnyClass::get(c"NSGlassEffectView").is_some() {
        return MaterialView::Glass(NSGlassEffectView::initWithFrame(
            NSGlassEffectView::alloc(mtm),
            frame,
        ));
    }
    let view = NSVisualEffectView::initWithFrame(NSVisualEffectView::alloc(mtm), frame);
    view.setMaterial(NSVisualEffectMaterial::Popover);
    view.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
    // 输入法进程从不在前台，跟随窗口激活状态的话材质会一直是失活的灰
    view.setState(NSVisualEffectState::Active);
    MaterialView::Blur(view)
}

/// 毛玻璃的圆角遮罩：一个 `2r + 1` 见方的圆角块，四角按 `r` 切成九宫格拉伸到视图大小。
fn rounded_mask(radius: f64) -> Retained<NSImage> {
    let side = radius * 2.0 + 1.0;
    let draw = RcBlock::new(move |rect: NSRect| -> Bool {
        NSColor::blackColor().set();
        NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(rect, radius, radius).fill();
        Bool::YES
    });
    let image =
        NSImage::imageWithSize_flipped_drawingHandler(NSSize::new(side, side), false, &draw);
    image.setCapInsets(NSEdgeInsets {
        top: radius,
        left: radius,
        bottom: radius,
        right: radius,
    });
    image.setResizingMode(NSImageResizingMode::Stretch);
    image
}
