//! 标记文本：preedit 里按分段画的拼音行，每段带种类（正常 / 纠错 / 未解析等）。

mod kind;
mod segment;

pub use kind::MarkedKind;
pub use segment::MarkedSegment;
