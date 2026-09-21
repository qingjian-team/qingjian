//! 注音解码与结果；保留既有公开路径。
mod cursor;
mod parse;
mod result;
mod unit;

pub use parse::decode;
pub(crate) use parse::decode_at;
pub use result::Decoded;
pub use unit::Unit;
