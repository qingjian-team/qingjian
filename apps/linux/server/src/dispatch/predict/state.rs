//! 等云端结果的节拍：请求发出后按短间隔来一次 `tick` 收，太久没回就当作这轮没有结果。
//! 防抖不在这里——`qingjian_predict::Worker` 在自己的线程里数 `debounce_ms`，壳只负责收。
//! 常数与 macOS 壳的 `PredictMonitor` 相同。

use std::time::{Duration, Instant};

/// 轮询间隔：一次网络请求几百毫秒，50 毫秒一次够快也不空转。
pub(super) const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// 最长等多久；`[predict] timeout_ms` 管的是请求本身，这里是壳等结果的兜底。
pub(super) const MAX_WAIT: Duration = Duration::from_secs(12);

/// 云联想结果的进行态。
#[derive(Debug, Default)]
pub(crate) struct PredictState {
    /// 本轮请求发出的时间；有它表示在等结果。
    polling_since: Option<Instant>,
}

impl PredictState {
    /// 请求已发出：开始等结果。
    pub(super) fn start_polling(&mut self) {
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

    /// 不等了（收到结果、组句结束、换会话）。
    pub(super) fn stop(&mut self) {
        self.polling_since = None;
    }

    /// 下一次该来 `tick` 的时长；没在等结果为 `None`。跨模块的节拍合并在 `Router::next_tick`。
    pub(crate) fn next_deadline(&self) -> Option<Duration> {
        self.polling_since.map(|_| POLL_INTERVAL)
    }
}
