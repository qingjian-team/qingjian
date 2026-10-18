//! 一个小时里云服务的请求与 Token 数。

use std::ops::AddAssign;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ApiUsageHour {
    /// 有结果的请求（收到响应或失败），还在等的不算。
    pub requests: u64,

    /// 网络、HTTP 或超时失败。
    pub failures: u64,

    pub input_tokens: u64,

    pub output_tokens: u64,

    pub total_tokens: u64,
}

impl AddAssign for ApiUsageHour {
    fn add_assign(&mut self, other: Self) {
        self.requests += other.requests;
        self.failures += other.failures;
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.total_tokens += other.total_tokens;
    }
}
