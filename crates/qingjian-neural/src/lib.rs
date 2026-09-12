//! 字级 Transformer 语言模型的本地推理（candle）。
//!
//! 加载训练仓库（`../train`）导出的模型——开发时是三件套目录（`model.safetensors` + `config.json` + `vocab.json`），
//! 随包与用户目录里是打成一个文件的 `.qjm`（见 [`qjm`]）——给「光标前文 + 候选文本」按字累加 log 概率，供 Core 给整句路径重打分。
//! 结构是 GPT-2 风格 decoder-only（pre-LN、erf GELU、可学习位置嵌入、输入输出嵌入共享），张量名与训练脚本约定一致。
//!
//! 缺省 CPU；`metal` feature 走 Apple GPU。

mod config;
mod core_scorer;
mod error;
mod model;
pub mod qjm;
mod scorer;
mod vocab;

pub use config::ModelConfig;
pub use error::NeuralError;
pub use model::CharLm;
pub use qjm::find_model;
pub use scorer::CharScorer;
pub use vocab::Vocab;
