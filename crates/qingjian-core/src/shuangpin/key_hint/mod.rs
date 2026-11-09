//! 键位提示：把一套双拼方案的键位表摊成一张键盘，按当前敲到哪一步标出每个键能拼成什么，给壳画在候选窗旁边。
//! 只给数据，什么时候弹出、怎么画由壳和渲染器决定。

mod cap;
mod hint;
mod state;

pub use cap::KeyCap;
pub use hint::KeyHint;
pub use state::KeyState;
