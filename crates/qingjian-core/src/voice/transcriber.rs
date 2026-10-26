//! 识别方。

use super::TranscriptEvent;

/// 把一段录音转成文字。实现可以联网或跑本地模型，但接口**不阻塞**：喂音频、收尾都立刻返回，结果由壳定时 [`Self::poll`]。
///
/// 一段识别的顺序是 [`Self::start`] → 若干次 [`Self::feed`] → [`Self::finish`]（说完了，等最终结果）或 [`Self::cancel`]（丢掉）。
pub trait Transcriber: Send {
    /// 现在能不能用（模型装好了、云端配好了）；`false` 时壳不开录音，入口提示还不能用。
    fn available(&self) -> bool;

    /// 开始新的一段，`sample_rate` 是之后喂进来的单声道 16 位 PCM 的采样率。上一段没收尾的直接作废。
    fn start(&mut self, sample_rate: u32);

    /// 一截录音（单声道 16 位 PCM）。
    fn feed(&mut self, samples: &[i16]);

    /// 说完了：不再有音频，之后会来一个 [`TranscriptEvent::Final`] 或 [`TranscriptEvent::Error`]。
    fn finish(&mut self);

    /// 丢掉这一段，之后不再有事件。
    fn cancel(&mut self);

    /// 取一件已经发生的事，没有返回 `None`。
    fn poll(&mut self) -> Option<TranscriptEvent>;
}

/// 不识别：还没有接任何识别方式时用，[`Transcriber::available`] 为 `false`。
#[derive(Debug, Clone, Copy, Default)]
pub struct NoTranscriber;

impl Transcriber for NoTranscriber {
    fn available(&self) -> bool {
        false
    }

    fn start(&mut self, _sample_rate: u32) {}

    fn feed(&mut self, _samples: &[i16]) {}

    fn finish(&mut self) {}

    fn cancel(&mut self) {}

    fn poll(&mut self) -> Option<TranscriptEvent> {
        None
    }
}
