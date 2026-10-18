//! 应用：使用方实现 [`App`]（视图函数 + 消息处理），[`Runner`] 管一帧的流程（建树、布局、交互、过渡）。
//! 开窗口的宿主（`host` 特性）与不开窗口的截图、测试都走 `Runner`，画出来的一样。

mod runner;
mod traits;
mod waker;

pub use runner::Runner;
pub use traits::App;
pub use waker::Waker;
