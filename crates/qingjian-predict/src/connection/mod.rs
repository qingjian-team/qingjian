//! 偏好设置「测试连接」：起线程发一条最小请求（`ConnectionTest`），结果是 `ConnectionReport`。

mod report;
mod test;

pub use report::ConnectionReport;
pub use test::ConnectionTest;
