use std::time::Duration;

/// [`super::Engine::annotate`] 的统计。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AnnotationReport {
    /// 尝试标注的候选数。
    pub total: usize,

    /// 查到译文的候选数。
    pub hits: usize,

    /// 查表总耗时。
    pub elapsed: Duration,
}
