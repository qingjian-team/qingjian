//! API 用量落盘：只累计请求与接口返回的 token 数；跨进程文件锁保护原子更新。

mod stats;
mod tokens;

#[cfg(test)]
mod tests;

pub use stats::ApiUsageStats;

/// 用户数据目录下的累计用量文件。
pub const API_USAGE_FILE: &str = "api-usage.json";
