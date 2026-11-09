//! 含章：自训的字级 Transformer（[`CharLm`]）与它的两种用法——给「光标前文 + 候选」打分的
//! [`CharScorer`]，与按拼音逐字生成整句的 [`P2c`]（Core 那头的适配是 [`P2cScorer`]）。
//! 模型开发时是三件套目录（`model.safetensors` + `config.json` + `vocab.json`），随包与用户目录里是打成一个文件的 `.qjm`（见 [`qjm`]）。
//! 结构是 GPT-2 风格 decoder-only（pre-LN、erf GELU、可学习位置嵌入、输入输出嵌入共享），张量名与训练脚本约定一致。

mod char_lm;
mod config;
mod core_scorer;
mod p2c;
pub mod qjm;
mod scorer;
mod vocab;

pub use char_lm::{CharLm, PrefixCache};
pub use config::ModelConfig;
pub use core_scorer::P2cScorer;
pub use p2c::{Candidate, P2c};
pub use qjm::find_model;
pub use scorer::CharScorer;
pub use vocab::Vocab;
