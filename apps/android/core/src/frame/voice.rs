//! 语音识别的一件事（中间结果、最终结果、出错），JSON 过 JNI 给壳。

use qingjian_core::TranscriptEvent;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct VoiceEvent {
    /// `partial` / `final` / `error`。
    pub kind: &'static str,

    /// 识别出的文字，出错时是给用户看的原因。
    pub text: String,
}

impl From<TranscriptEvent> for VoiceEvent {
    fn from(event: TranscriptEvent) -> Self {
        let (kind, text) = match event {
            TranscriptEvent::Partial(text) => ("partial", text),
            TranscriptEvent::Final(text) => ("final", text),
            TranscriptEvent::Error(text) => ("error", text),
        };
        Self { kind, text }
    }
}
