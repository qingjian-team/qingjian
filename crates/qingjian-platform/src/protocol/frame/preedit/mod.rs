//! 组句拼音行（preedit）的分段模型：一段文本（[`PreeditSegment`]）加它的种类（[`PreeditKind`]）。

mod kind;
mod segment;

pub use kind::PreeditKind;
pub use segment::PreeditSegment;
