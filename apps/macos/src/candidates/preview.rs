//! 设置页内的中英双向字体预览，复用候选窗口的视图与绘制路径。

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSColor, NSView,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use qingjian_platform::{GeneralConfig, LayoutMode, ThemeMode};

use super::colors::CandidateColors;
use super::frame::Frame;
use super::row::{Row, Tone};
use super::theme::Theme;
use super::typography::Typography;
use super::view::CandidateView;

/// 留足 48 磅字体的行高；调整字号时不推动下方控件。
pub(crate) const PREVIEW_HEIGHT: f64 = 184.0;

pub(crate) struct CandidatePreview {
    container: Retained<NSView>,

    candidate: Retained<CandidateView>,
}

impl CandidatePreview {
    pub fn new(mtm: MainThreadMarker, width: f64) -> Self {
        let container = NSView::initWithFrame(
            mtm.alloc(),
            NSRect::new(NSPoint::ZERO, NSSize::new(width, PREVIEW_HEIGHT)),
        );
        let candidate = CandidateView::new(mtm, Theme::system_default());
        // 字体示例始终并排显示两组文字，方便比较各自的改动。
        candidate.set_layout(LayoutMode::Vertical);
        container.addSubview(&candidate);
        Self {
            container,
            candidate,
        }
    }

    pub fn view(&self) -> &NSView {
        &self.container
    }

    pub fn color_swatches(&self) -> [Retained<NSColor>; 6] {
        self.candidate.color_swatches()
    }

    pub fn appearance(&self) -> Retained<NSAppearance> {
        self.candidate.effectiveAppearance()
    }

    pub fn sync(&self, general: &GeneralConfig) {
        // SAFETY: 只读 AppKit 导出的常量名。
        let name = unsafe {
            match general.theme {
                ThemeMode::System => None,
                ThemeMode::Light => Some(NSAppearanceNameAqua),
                ThemeMode::Dark => Some(NSAppearanceNameDarkAqua),
            }
        };
        let appearance = name.and_then(NSAppearance::appearanceNamed);
        self.candidate.setAppearance(appearance.as_deref());
        self.candidate.set_typography(&Typography::from(general));
        self.candidate.set_renderer(general.renderer);
        self.candidate.set_colors(&CandidateColors::from(general));
        // 固定示例只用于显示，不经过输入引擎，也不读取或记录用户输入。
        let frame = Frame {
            rows: vec![
                Row {
                    index: "1".into(),
                    text: "学习".into(),
                    foreign_text: false,
                    chinese_annotation: false,
                    annotation: vec![
                        ("v. ".into(), Tone::PartOfSpeech),
                        ("learn".into(), Tone::Fresh),
                    ],
                    cloud: false,
                },
                Row {
                    index: "2".into(),
                    text: "learn".into(),
                    foreign_text: true,
                    chinese_annotation: true,
                    annotation: vec![
                        ("v. ".into(), Tone::PartOfSpeech),
                        ("学习".into(), Tone::Gloss),
                    ],
                    cloud: false,
                },
            ],
            ..Frame::default()
        };
        let size = self.candidate.set_frame(&frame);
        let origin = NSPoint::new(0.0, ((PREVIEW_HEIGHT - size.height) / 2.0).floor().max(0.0));
        self.candidate.setFrame(NSRect::new(origin, size));
    }
}
