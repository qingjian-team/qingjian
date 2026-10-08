//! 候选窗口：自绘 NSPanel 与视图、一帧的数据（preedit / 行 / 页脚）、主题。

mod bitmap;
mod cloud_icon;
pub(crate) mod colors;
mod frame;
mod native_font;
mod preedit;
mod preview;
mod row;
mod theme;
pub(crate) mod typography;
mod view;
mod window;

pub(crate) use bitmap::available_families;
pub use frame::Frame;
pub use preedit::Preedit;
pub(crate) use preview::{CandidatePreview, PREVIEW_HEIGHT};
pub use row::Row;
pub use window::CandidateWindow;
