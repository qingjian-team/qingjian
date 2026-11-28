//! 热加载任务不等待输入线程；结果通道只容纳一批已读好的资源。

use std::io;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Instant;

use qingjian_platform::SchemeSlot;

use super::{ConfigReload, prepared::PreparedReload};

pub(crate) struct ReloadTask {
    /// 输入线程以 `try_recv` 领取结果，绝不等待文件系统。
    receiver: Receiver<PreparedReload>,
}

impl ReloadTask {
    pub(super) fn spawn(
        reload: ConfigReload,
        slot: SchemeSlot,
        code_loaded: bool,
    ) -> io::Result<Self> {
        let (sender, receiver) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name("qingjian-reload".into())
            .spawn(move || {
                let started = Instant::now();
                let prepared = PreparedReload::load(reload, slot, code_loaded);
                let elapsed_ms = started.elapsed().as_millis();
                if elapsed_ms >= 100 {
                    tracing::warn!(elapsed_ms, "configuration scan and loading slow");
                }
                let _ = sender.send(prepared);
            })?;
        Ok(Self { receiver })
    }

    pub(super) fn poll(&self) -> Result<PreparedReload, TryRecvError> {
        self.receiver.try_recv()
    }
}
