//! 按键事件：交互层认得的键，与一次按键（键 + 修饰键）。

mod code;
mod press;

pub use code::KeyCode;
pub use press::KeyPress;
