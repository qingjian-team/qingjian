//! 语音识别的跨平台接口：云端与本地离线模型这类各平台都能用的识别方式实现 [`Transcriber`]，壳只管录音与界面。
//!
//! 系统自带的识别（Android `SpeechRecognizer`、iOS `SFSpeechRecognizer`）留在各平台壳里；
//! 不论哪种方式，识别出的文字都经 [`crate::Engine::commit_voice`] 上屏。设计见 `docs/design/voice-and-swipe.md`。

mod event;
mod transcriber;

pub use event::TranscriptEvent;
pub use transcriber::{NoTranscriber, Transcriber};
