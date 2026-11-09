//! 语音：识别结果上屏（各种识别方式共用），以及 Rust 侧识别方（云端、离线模型）的录音进出。

use qingjian_core::{NoTranscriber, Transcriber, TranscriptEvent};

use super::Session;
use crate::frame::Frame;
use crate::voice::VoiceSettings;

impl Session {
    /// 按设置页「语音识别」那份换识别方；`None`（没配）退回 `NoTranscriber`。
    pub fn set_voice(&mut self, settings: Option<&VoiceSettings>) {
        let transcriber = settings.map_or_else(
            || Box::new(NoTranscriber) as Box<dyn Transcriber>,
            VoiceSettings::transcriber,
        );
        self.set_transcriber(transcriber);
    }

    /// 换识别方；不用 Rust 侧识别时是 `NoTranscriber`。
    pub fn set_transcriber(&mut self, transcriber: Box<dyn Transcriber>) {
        self.transcriber.cancel();
        self.transcriber = transcriber;
    }

    /// Rust 侧的识别方能不能用；壳据此决定走不走这条路。
    pub fn voice_available(&self) -> bool {
        self.transcriber.available()
    }

    pub fn voice_start(&mut self, sample_rate: u32) {
        self.transcriber.start(sample_rate);
    }

    pub fn voice_feed(&mut self, samples: &[i16]) {
        self.transcriber.feed(samples);
    }

    pub fn voice_finish(&mut self) {
        self.transcriber.finish();
    }

    pub fn voice_cancel(&mut self) {
        self.transcriber.cancel();
    }

    pub fn poll_voice(&mut self) -> Option<TranscriptEvent> {
        self.transcriber.poll()
    }

    /// 识别出的一段文字上屏：Core 转繁体、记历史与统计。组句中（壳没先落下）把敲的字母原样落在前面。
    pub fn commit_voice(&mut self, text: &str) -> Frame {
        let mut commit = if self.engine.composition().is_empty() {
            String::new()
        } else {
            self.engine.take_raw()
        };
        if let Some(kana) = &mut self.kana {
            commit.push_str(&kana.hiragana());
            kana.clear();
        }
        commit.push_str(&self.engine.commit_voice(text));
        self.refresh(commit)
    }
}
