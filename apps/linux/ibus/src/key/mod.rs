//! IBus 的按键 → 协议里的 [`qingjian_platform::protocol::KeyEvent`]。
//!
//! 这一层只做翻译，不做判断：哪个键翻页、哪个键上屏、要不要吃掉，全由 Server 决定
//! （`docs/contributing.md`「架构约束」：平台层里不允许出现排序逻辑与文本变换）。

pub mod keysym;
pub mod mapping;
pub mod modifiers;

pub use mapping::to_key_event;
pub use modifiers::is_release;
