//! 候选窗口：自绘 NSPanel 与视图、一帧的数据（preedit / 行 / 页脚）、主题。

mod animation;
mod backdrop;
mod bitmap;
mod bounds;
mod cloud_icon;
mod frame;
mod key_hint;
mod menu;
mod panel;
mod preedit;
mod row;
mod theme;
mod view;
mod window;

pub(crate) use bitmap::{available_families, family_files};
pub use frame::Frame;
pub use panel::PanelMouse;
pub use preedit::Preedit;
pub use row::Row;
pub use view::MenuHandlers;
pub use window::CandidateWindow;
