//! 注入与开关（词库的在 `dictionaries.rs`）：模糊音、双拼、翻译 / 学习 / 联想的挂接，以及相应的只读访问。
//!
//! `Engine` 的注入方法多，按职责拆成子模块：`schemes` 输入方案与按键规则、`decode` 键解码、
//! `predict` 联想 / 重打分 / 语言模型、`english` 英文与翻译、`runtime` 日志 / 统计 / 面板、
//! `punctuation` 标点与短语、`shuangpin` 双拼、`aux_code` 辅码。

mod aux_code;
mod decode;
mod english;
mod learning;
mod predict;
mod punctuation;
mod runtime;
mod schemes;
mod shuangpin;
