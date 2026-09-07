use std::time::Duration;

/// 连通性测试成功时的结果。
pub struct ConnectionReport {
    /// 请求用的模型名。
    pub model: String,

    /// 从发出到收到回复的时间。
    pub elapsed: Duration,

    /// 模型的原始回复（只用来在日志里对一眼）。
    pub reply: String,
}
