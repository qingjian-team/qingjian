//! 叫醒窗口：后台线程（网络请求、文件扫描）做完了，从任意线程调 [`Waker::wake`]，宿主就在界面线程上调 [`super::App::wake`]。

use std::sync::Arc;

#[derive(Clone)]
pub struct Waker(Arc<dyn Fn() + Send + Sync>);

impl Waker {
    pub fn new(wake: impl Fn() + Send + Sync + 'static) -> Self {
        Self(Arc::new(wake))
    }

    /// 什么都不做：不开窗口时（截图、测试）用。
    pub fn noop() -> Self {
        Self::new(|| {})
    }

    pub fn wake(&self) {
        (self.0)();
    }
}

impl std::fmt::Debug for Waker {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Waker")
    }
}
