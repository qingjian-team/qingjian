//! 带时间衰减的计数：牛顿冷却。选择次数只增不减时，半年前的临时偏好会永久占着候选位；
//! 按「N 天不用就减半」冷却，权重跟着现在的习惯走。粒度是天（unix 天数），落盘带最近使用日，
//! 加载时按它补算衰减，进程重启不重置。

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// 缺省半衰期（天）：90 天不用，计数减半。回放调参可用 `--tune decay-days=` 覆盖，0 为不衰减。
pub const DEFAULT_HALF_LIFE_DAYS: f64 = 90.0;

/// 冷了多少天算是「真的不用了」：落盘时把最近使用日在这一年之外的条目清出。
/// 衰减只影响排序权重，记录本身不清——只选过一两次的词冷却之后还留在文件里，别让用户觉得学过的词被忘掉了。
pub const COLD_DAYS: u64 = 365;

/// 今天是 unix 纪元起第几天。UTC 的「天」与本地时区差半天，对按天冷却没有影响。
pub fn today() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() / 86_400)
}

/// 文件的修改时间换算成 unix 天；读不到（新建的文件）就当今天。
/// 旧格式文件里没有最近使用日，加载时拿它当整份文件的缺省日。
pub fn mtime_day(path: &Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or_else(today, |d| d.as_secs() / 86_400)
}

/// 一条会冷却的计数：值与最近一次使用的日期。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decay {
    /// 累计次数（只被记录与撤销改动，衰减不写回它）。
    pub count: u32,

    /// 最近一次使用的 unix 天数。
    pub last_used: u64,
}

impl Decay {
    /// 从落盘的 (次数, 天数) 还原。
    pub fn from_parts(count: u32, last_used: u64) -> Self {
        Self { count, last_used }
    }

    /// 记 `times` 次，刷新最近使用日。
    pub fn record(&mut self, times: u32, now: u64) {
        self.count = self.count.saturating_add(times);
        self.last_used = now.max(self.last_used);
    }

    /// 退回最多 `times` 次（撤销），返回实际退回的份数；减到零由调用方删条目。
    pub fn unrecord(&mut self, times: u32) -> u32 {
        let removed = times.min(self.count);
        self.count -= removed;
        removed
    }

    /// 有效次数：`count × 2^(−间隔天数 / 半衰期)` 四舍五入。冷却到 0 的条目查询时已不起作用，
    /// 记录仍留着，冷满 [`COLD_DAYS`] 才在落盘时清出。`half_life_days` 非正数视为不衰减。
    pub fn effective(&self, now: u64, half_life_days: f64) -> u32 {
        if half_life_days <= 0.0 {
            return self.count;
        }
        let days = now.saturating_sub(self.last_used) as f64;
        (f64::from(self.count) * (-days / half_life_days).exp2()).round() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_decays_by_half_life_and_floor_is_zero() {
        let entry = Decay::from_parts(8, 100);
        assert_eq!(entry.effective(100, 30.0), 8);
        assert_eq!(entry.effective(130, 30.0), 4);
        assert_eq!(entry.effective(160, 30.0), 2);
        assert_eq!(entry.effective(190, 30.0), 1);
        // 恰好 0.5 按「四舍五入远离零」进位：120 天还是 1，再老 10 天才是 0
        assert_eq!(entry.effective(220, 30.0), 1);
        assert_eq!(entry.effective(230, 30.0), 0);
        // 未来日期（时钟回拨）不放大计数
        assert_eq!(entry.effective(90, 30.0), 8);
    }

    #[test]
    fn zero_half_life_disables_decay() {
        let entry = Decay::from_parts(3, 1);
        assert_eq!(entry.effective(100_000, 0.0), 3);
        assert_eq!(entry.effective(100_000, -1.0), 3);
    }

    #[test]
    fn record_refreshes_last_used_and_unrecord_removes() {
        let mut entry = Decay::from_parts(5, 100);
        entry.record(1, 200);
        assert_eq!(entry.count, 6);
        assert_eq!(entry.last_used, 200);
        // 时钟回拨时不往回改
        entry.record(1, 150);
        assert_eq!(entry.last_used, 200);
        assert_eq!(entry.unrecord(2), 2);
        assert_eq!(entry.count, 5);
        assert_eq!(entry.unrecord(9), 5);
        assert_eq!(entry.count, 0);
    }
}
