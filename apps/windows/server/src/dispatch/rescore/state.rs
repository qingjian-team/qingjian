//! 本地整句模型的两个节拍：停键后的防抖（到点才把整句路径送去后台打分）与结果轮询。
//! Server 没有定时器，工人循环按 [`RescoreState::next_deadline`] 给的时长等消息，超时就来一次 `tick`；
//! 轮询间隔与 macOS 壳的 `RescoreMonitor` 相同；防抖比它短，结果到了后台线程还会直接叫醒工人循环（[`crate::ipc::Work::Wake`]）。

use std::time::{Duration, Instant};

/// 停键多久才请求重排：要短到停键后很快换好首选，又要让连着敲（键间隔百来毫秒）时出字、打分两轮在下一键前算完、不排队。
/// 各段耗时与取值见 `docs/notes/crate-notes.md`（Windows Server 一节）。
/// 后台线程排队时只算最新一条，敲得再快也不会越积越多。
pub(super) const DEBOUNCE: Duration = Duration::from_millis(30);

/// 轮询间隔：模型一次几十毫秒。
pub(super) const POLL_INTERVAL: Duration = Duration::from_millis(20);

/// 最长等多久；后台线程卡住时兜底。
pub(super) const MAX_WAIT: Duration = Duration::from_secs(2);

/// 重排的进行态。
#[derive(Debug, Default)]
pub(crate) struct RescoreState {
    /// 最近一次缓冲变化的时间；有它表示在等防抖到点。
    wanted_since: Option<Instant>,

    /// 本轮请求发出的时间；有它表示在等后台结果。
    polling_since: Option<Instant>,
}

impl RescoreState {
    /// 又敲了一键：重新计时。
    pub(super) fn schedule(&mut self) {
        self.wanted_since = Some(Instant::now());
    }

    /// 防抖到点了没。
    pub(super) fn debounce_elapsed(&self) -> bool {
        self.wanted_since
            .is_some_and(|since| since.elapsed() >= DEBOUNCE)
    }

    /// 请求已发出：开始等结果。
    pub(super) fn start_polling(&mut self) {
        self.wanted_since = None;
        self.polling_since = Some(Instant::now());
    }

    pub(super) fn polling(&self) -> bool {
        self.polling_since.is_some()
    }

    /// 等结果等太久了。
    pub(super) fn expired(&self) -> bool {
        self.polling_since
            .is_some_and(|since| since.elapsed() > MAX_WAIT)
    }

    /// 什么都不等。
    pub(super) fn stop(&mut self) {
        self.wanted_since = None;
        self.polling_since = None;
    }

    /// 下一次该来 `tick` 的时长；什么都不等为 `None`。
    pub(super) fn next_deadline(&self) -> Option<Duration> {
        if self.polling_since.is_some() {
            return Some(POLL_INTERVAL);
        }
        self.wanted_since
            .map(|since| DEBOUNCE.saturating_sub(since.elapsed()))
    }
}
