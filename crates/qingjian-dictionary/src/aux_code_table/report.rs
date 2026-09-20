//! 一次码表导入的统计口径。

/// 一次码表导入的统计。与词库导入不同，「有词无码」不静默跳过：码表少一行就是少一个词可用。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AuxCodeTableImportReport {
    /// 读入的正文行数（不含注释、空行与前导的 YAML 头）。
    pub read: usize,

    /// 成功建成条目的行数。
    pub with_code: usize,

    /// 有词但没有码的行数（码列为空，或表里根本没有码列）。
    pub no_code: usize,

    /// 码非法（不是 a-z、或长度不在 1–8）被丢掉的行数。
    pub skipped: usize,
}
