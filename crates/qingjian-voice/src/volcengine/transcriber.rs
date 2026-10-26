//! 豆包流式语音识别的 [`Transcriber`] 实现：每段识别开一个后台线程（[`super::stream`]），这边只往通道里放录音、取事件。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, TryRecvError};
use std::thread;

use qingjian_core::{Transcriber, TranscriptEvent};

use super::VolcengineConfig;
use super::command::Command;
use super::running::Running;
use super::stream;
use crate::VoiceError;

pub struct VolcengineTranscriber {
    config: VolcengineConfig,

    current: Option<Running>,
}

impl VolcengineTranscriber {
    pub fn new(config: VolcengineConfig) -> Self {
        Self {
            config,
            current: None,
        }
    }
}

impl Transcriber for VolcengineTranscriber {
    fn available(&self) -> bool {
        self.config.ready()
    }

    fn start(&mut self, sample_rate: u32) {
        self.cancel();
        let (commands, command_receiver) = mpsc::channel();
        let (event_sender, events) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        let config = self.config.clone();
        let flag = Arc::clone(&cancelled);
        let spawned = thread::Builder::new()
            .name("qingjian-voice".to_owned())
            .spawn(move || stream::run(config, sample_rate, command_receiver, event_sender, flag));
        match spawned {
            Ok(_) => {
                self.current = Some(Running {
                    commands,
                    events,
                    cancelled,
                });
            }
            Err(error) => tracing::error!(%error, "开不了语音识别线程"),
        }
    }

    fn feed(&mut self, samples: &[i16]) {
        if let Some(running) = &self.current {
            let _ = running.commands.send(Command::Audio(samples.to_vec()));
        }
    }

    fn finish(&mut self) {
        if let Some(running) = &self.current {
            let _ = running.commands.send(Command::Finish);
        }
    }

    fn cancel(&mut self) {
        if let Some(running) = self.current.take() {
            running.cancelled.store(true, Ordering::Relaxed);
        }
    }

    fn poll(&mut self) -> Option<TranscriptEvent> {
        let running = self.current.as_ref()?;
        match running.events.try_recv() {
            Ok(event) => {
                if !matches!(event, TranscriptEvent::Partial(_)) {
                    self.current = None;
                }
                Some(event)
            }
            Err(TryRecvError::Empty) => None,
            // 线程没交结果就走了（不该发生）：当出错收尾，壳好关面板
            Err(TryRecvError::Disconnected) => {
                self.current = None;
                Some(TranscriptEvent::Error(VoiceError::Closed.describe()))
            }
        }
    }
}

impl Drop for VolcengineTranscriber {
    fn drop(&mut self) {
        self.cancel();
    }
}
