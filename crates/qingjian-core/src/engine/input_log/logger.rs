use super::InputLogEntry;

/// 输入日志的落盘方。实现放学习 crate，Core 只往里送条目。
pub trait InputLogger: Send {
    /// 记一条。实现不能阻塞输入：写失败只记日志。
    fn record(&mut self, entry: InputLogEntry);

    /// 把缓冲写出去。壳在停用输入法时调用。
    fn flush(&mut self) {}

    /// 有没有在记（缺省实现没有，壳据此在诊断信息里说明）。
    fn is_enabled(&self) -> bool {
        true
    }
}

/// 不记。
#[derive(Debug, Clone, Copy, Default)]
pub struct NoInputLogger;

impl InputLogger for NoInputLogger {
    fn record(&mut self, _entry: InputLogEntry) {}

    fn is_enabled(&self) -> bool {
        false
    }
}
