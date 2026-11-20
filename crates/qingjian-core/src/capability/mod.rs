//! 注入点收敛层：把分散挂在 [`Engine`](crate::engine::Engine) 各字段上的可选能力
//! （翻译、联想、学习、语言模型、打分器……）登记到同一个 [`Capabilities`] 注册表。
//!
//! 本轮是纯增量：Engine 现有字段、构造函数与注入方法一律不动，注册表只负责
//! 「这种能力有没有、取回的是哪种类型」。待 Engine 按职责拆分（refactor/setup-split）
//! 完成后，再把各 trait 对象字段逐个搬进这里；迁移清单见本轮改造报告的遗留项一节。

mod registry;

pub use registry::{Capabilities, CapabilityKind};
