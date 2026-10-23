//! 自绘 UI 引擎：场景树（taffy 布局 + 每个节点画什么）→ 位图，各平台只负责把位图贴到窗口上。
//!
//! 栅格用 tiny-skia（纯 CPU），文字用 cosmic-text（fontdb 选字体 + 整形 + swash 栅格，带回退链与彩色 emoji）。
//! 字体不扫系统目录，按平台清单只加载界面字体、中文、日文、emoji 几个文件（[`FontLibrary`]）。
//! 场景树里一律是像素，点数由使用方乘好倍数；输出是预乘 alpha 的 RGBA 位图（[`Painted`]）。

mod animation;
mod app;
mod canvas;
mod color;
mod error;
mod fonts;
#[cfg(feature = "host")]
mod host;
mod scene;
mod shadow;
mod stage;
mod svg_image;
mod text;
mod ui;
mod widgets;

pub use animation::{Easing, Keyframe, Keyframes};
pub use app::{App, Runner, Waker};
pub use canvas::Canvas;
pub use color::Color;
pub use error::RenderError;
/// Windows 的字体登记：按字族名找文件、列字族名。
#[cfg(target_os = "windows")]
pub use fonts::directwrite as system_fonts;
pub use fonts::{FontLibrary, UiFont};
#[cfg(feature = "host")]
pub use host::{HostError, TITLEBAR, WindowOptions, run as run_window, set_clipboard};
pub use scene::{BoxPaint, Effect, EffectKind, Fill, Icon, Scene, Visual};
pub use stage::{Painted, Stage};
pub use svg_image::SvgImage;
pub use text::font::{FamilyId, FamilyList, FamilyTable, FontSpec, FontWeight};
pub use text::{Decoration, TextPainter, TextSize, TextStyle};
pub use ui::{
    ElementState, Elements, Event, Interact, Interaction, Key, KeyCode, KeyPress, Modifiers,
    Response, ScrollExtent, Searchable, TextInput, Ui,
};
pub use widgets::{
    BarChart, Button, Checkbox, Datum, Edit, Heatmap, KeyRecorder, Menu, NavItem, PopupMenu,
    ProgressBar, ScrollArea, SearchField, Segmented, Select, SettingRow, Slider, Stepper, Switch,
    TextEdit, TextField, WidgetColors, WidgetTheme, dialog, group, info, info_with_link, paragraph,
    section_title,
};

/// 使用方建树用的布局类型、自己画图标用的路径与混合模式，与这里是同一个 taffy、tiny-skia。
pub use taffy;
pub use tiny_skia;
pub use tiny_skia::Pixmap;
