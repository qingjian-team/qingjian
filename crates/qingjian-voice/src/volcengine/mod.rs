//! 火山引擎豆包流式语音识别（大模型流式识别 WebSocket 接口，v3 二进制协议）。
//! 一段识别一条连接：开始说话时连上发配置，边说边发约 200 ms 一包的 PCM，说完发最后一包、等最终结果。
//! 接口文档：<https://www.volcengine.com/docs/6561/1354869>。

mod command;
mod config;
mod protocol;
mod reply;
mod running;
mod stream;
mod transcriber;

pub use config::VolcengineConfig;
pub use transcriber::VolcengineTranscriber;

use std::thread;
use std::time::{Duration, Instant};

use qingjian_core::{Transcriber, TranscriptEvent};

/// 设置页「测试连接」：用这份配置识别半秒静音，阻塞到有结果。成功回用时，失败回给用户看的原因。
pub fn test_connection(config: &VolcengineConfig) -> Result<Duration, String> {
    let started = Instant::now();
    let mut transcriber = VolcengineTranscriber::new(config.clone());
    if !transcriber.available() {
        return Err(crate::VoiceError::MissingApiKey.describe());
    }
    transcriber.start(SAMPLE_RATE);
    transcriber.feed(&[0; SAMPLE_RATE as usize / 2]);
    transcriber.finish();
    while started.elapsed() < TEST_TIMEOUT {
        match transcriber.poll() {
            Some(TranscriptEvent::Final(_)) => return Ok(started.elapsed()),
            Some(TranscriptEvent::Error(message)) => return Err(message),
            Some(TranscriptEvent::Partial(_)) | None => thread::sleep(Duration::from_millis(20)),
        }
    }
    transcriber.cancel();
    Err(crate::VoiceError::Timeout(TEST_TIMEOUT.as_millis() as u64).describe())
}

const SAMPLE_RATE: u32 = 16_000;

const TEST_TIMEOUT: Duration = Duration::from_secs(12);
