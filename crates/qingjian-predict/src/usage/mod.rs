//! API 用量落盘：只累计请求与接口返回的 token 数，另按小时记一份给统计页画图；跨进程文件锁保护原子更新。

mod hour;
mod hours;
mod stats;
mod tokens;

#[cfg(test)]
mod tests;

pub use hour::ApiUsageHour;
pub use hours::{API_USAGE_HOURLY_FILE, ApiUsageHours};
pub use stats::ApiUsageStats;

/// 用户数据目录下的累计用量文件。
pub const API_USAGE_FILE: &str = "api-usage.json";
