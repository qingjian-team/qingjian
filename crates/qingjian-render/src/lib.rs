//! 自绘渲染器：把候选窗的一帧（拼音行、候选行、页脚）按主题画成一张位图，各平台壳只负责把位图贴到窗口上。
//!
//! 栅格用 tiny-skia（纯 CPU），文字用 cosmic-text（fontdb 选字体 + 整形 + swash 栅格，带回退链与彩色 emoji）。
//! 字体不扫系统目录，按平台清单只加载界面字体、中文、日文、emoji 几个文件（[`FontLibrary`]）。
//! 所有尺寸以「点」为单位写在 [`Theme`] 里，[`Renderer`] 按缩放倍数换成像素；输出是预乘 alpha 的 RGBA 位图。
//!
//! 设计与验收见 `docs/design/rendering.md`。

mod animation;
mod canvas;
mod cloud;
mod color;
mod error;
mod fonts;
mod frame;
mod gear;
mod layout;
mod renderer;
mod scene;
mod shadow;
mod text;
mod theme;

pub use color::Color;
pub use error::RenderError;
/// Windows 的字体登记：按字族名找文件、列字族名（设置页用）。
#[cfg(target_os = "windows")]
pub use fonts::directwrite as system_fonts;
pub use fonts::{FontLibrary, UiFont};
pub use frame::{Frame, Mode, Preedit, PreeditSegment, PreeditStyle, Row, Tone};
pub use layout::Layout;
pub use renderer::{HitRegion, HitTarget, Rendered, RenderedStatus, Renderer, StatusCell};
pub use theme::{FontSpec, TextSizes, Theme, ThemeError, ThemeLibrary};

/// 让 `tiny_skia::Pixmap` 的使用方不用再单独依赖 tiny-skia。
pub use tiny_skia::Pixmap;
