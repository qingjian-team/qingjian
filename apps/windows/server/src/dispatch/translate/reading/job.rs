//! 一个可取消、带超时的备用读取请求；取消不会把迟到结果交给下一次翻译。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

pub(super) type ReadResult = Result<String, &'static str>;

pub(in crate::dispatch::translate) struct ReadJob {
    result: Receiver<ReadResult>,

    cancelled: Arc<AtomicBool>,

    started: Instant,
}

impl ReadJob {
    pub(in crate::dispatch::translate) fn start(process: u32, window: u64) -> Self {
        let (sender, result) = std::sync::mpsc::sync_channel(1);
        let cancelled = Arc::new(AtomicBool::new(false));
        #[cfg(all(windows, not(test)))]
        super::native::spawn(process, window, cancelled.clone(), sender);
        #[cfg(any(not(windows), test))]
        {
            let _ = (process, window);
            let _ = sender.send(Err("当前平台不支持备用选区读取"));
        }
        Self {
            result,
            cancelled,
            started: Instant::now(),
        }
    }

    pub(in crate::dispatch::translate) fn poll(&self) -> Option<ReadResult> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Disconnected) => Some(Err("备用读取已中断")),
            Err(TryRecvError::Empty) if self.started.elapsed() > Duration::from_secs(5) => {
                self.cancelled.store(true, Ordering::Relaxed);
                Some(Err("读取选中文字超时，请按 Esc 退出"))
            }
            Err(TryRecvError::Empty) => None,
        }
    }

    #[cfg(test)]
    pub(in crate::dispatch::translate) fn pending() -> (
        Self,
        std::sync::mpsc::SyncSender<ReadResult>,
        Arc<AtomicBool>,
    ) {
        let (sender, result) = std::sync::mpsc::sync_channel(1);
        let cancelled = Arc::new(AtomicBool::new(false));
        (
            Self {
                result,
                cancelled: cancelled.clone(),
                started: Instant::now(),
            },
            sender,
            cancelled,
        )
    }
}

impl Drop for ReadJob {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};

    use super::ReadJob;

    #[test]
    fn timeout_and_cancellation_discard_late_results() {
        let (mut job, sender, cancelled) = ReadJob::pending();
        assert!(job.poll().is_none());
        job.started = Instant::now() - Duration::from_secs(6);
        assert!(job.poll().unwrap().is_err());
        assert!(cancelled.load(Ordering::Relaxed));
        drop(job);
        assert!(sender.send(Ok("late".into())).is_err());
    }
}
