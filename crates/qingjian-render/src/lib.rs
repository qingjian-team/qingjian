//! 自绘渲染器：把候选窗的一帧（拼音行、候选行、页脚）按主题画成一张位图，各平台壳只负责把位图贴到窗口上。
//!
//! 画在 `bamboo-slip` 上：这里只管候选窗自己的事——帧数据、主题格式、按主题模板建场景树、点击区域与状态条；
//! 栅格、文字、字体、布局与动画都在引擎里。所有尺寸以「点」为单位写在 [`Theme`] 里，[`Renderer`] 按缩放倍数换成像素。
//!
//! 设计与验收见 `docs/design/rendering.md`。

mod frame;
mod icon;
mod layout;
pub mod preview;
mod renderer;
pub mod sample;
mod theme;

/// Windows 的字体登记：按字族名找文件、列字族名（设置页用）。
#[cfg(target_os = "windows")]
pub use bamboo_slip::system_fonts;
pub use bamboo_slip::{Color, FontLibrary, FontSpec, Pixmap, RenderError, UiFont};
pub use frame::{Frame, Mode, Preedit, PreeditSegment, PreeditStyle, Row, Tone};
pub use layout::Layout;
pub use renderer::{HitRegion, HitTarget, Rendered, RenderedStatus, Renderer, StatusCell};
pub use theme::{TextSizes, Theme, ThemeError, ThemeLibrary};

mod translation;
pub use translation::translation_lines;
