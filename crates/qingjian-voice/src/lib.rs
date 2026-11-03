//! 云端语音识别：实现 Core 的 [`qingjian_core::Transcriber`]，录音由壳喂进来，这里连云端、收结果。
//! 现在接的是火山引擎豆包流式语音识别（[`volcengine`]）；Core 不联网，所以放在这个独立 crate。
//! 设计见 `docs/design/voice-and-swipe.md`。

#![forbid(unsafe_code)]

mod error;
mod tls;
pub mod volcengine;

pub use error::VoiceError;
pub use volcengine::{VolcengineConfig, VolcengineTranscriber};
