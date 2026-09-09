//! 按键相关的协议类型：一次按键（[`KeyEvent`]）与 Server 的处置（[`KeyOutcome`]）。

mod event;
mod outcome;

pub use event::KeyEvent;
pub use outcome::KeyOutcome;
