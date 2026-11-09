//! 按键相关的协议类型：一次按键（[`KeyEvent`]）、它的修饰键（[`KeyModifiers`]）、Server 的处置（[`KeyOutcome`]）。

mod caret_edit;
mod event;
mod modifiers;
mod outcome;

pub use caret_edit::CaretEdit;
pub use event::KeyEvent;
pub use modifiers::KeyModifiers;
pub use outcome::KeyOutcome;
