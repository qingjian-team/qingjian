//! 双拼：两键一音节，第一键声母、第二键韵母，零声母另有约定。
//!
//! 解码只做一件事：把敲的键翻成全拼（音节之间用 `'` 连上，切分因此没有歧义），
//! 之后的切分、查词、整句、联想全部复用全拼的那一套；壳与词库都不知道双拼的存在。
//! 上屏消耗按「音节对应几个键」换算回缓冲区（见 [`Decoded::keys_for`]）。
//!
//! 四套方案的键位表按 Rime 的 `double_pinyin*.schema.yaml` 核对（搜狗方案来自 rime-ice 的整理），
//! 见 [`Scheme`] 的各表；每套方案对全部音节做往返测试。

mod decoded;
mod scheme;
mod table;
mod unit;

pub use decoded::Decoded;
pub use scheme::Scheme;
pub use unit::Unit;
