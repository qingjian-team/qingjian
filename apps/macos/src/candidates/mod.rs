//! 候选窗口：自绘 NSPanel 与视图、一帧的数据（preedit / 行 / 页脚）、主题。

mod cloud_icon;
mod frame;
mod preedit;
mod row;
mod theme;
mod view;
mod window;

pub use frame::Frame;
pub use preedit::Preedit;
pub use row::Row;
pub use window::CandidateWindow;
