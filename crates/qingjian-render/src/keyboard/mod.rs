//! 键位提示键盘要画的内容：几排键，每个键的键名、角标、下方几行小字（双拼）或一格格字根（五笔），与状态。只是展示形态，键位从哪来由壳决定。

mod board;
mod keycap;
mod root;
mod state;

pub use board::Keyboard;
pub use keycap::Keycap;
pub use root::Root;
pub use state::KeycapState;
