use super::{InputLogEntry, InputLogger};

/// 套在壳装配的输入日志外面的一层：私密输入期间一条都不记（条目里有敲的键与上屏的字），见 [`crate::Engine::set_private`]。
pub(in crate::engine) struct MutedLogger {
    inner: Box<dyn InputLogger>,

    /// 私密中：`record` 直接丢。
    muted: bool,
}

impl MutedLogger {
    pub(in crate::engine) fn new(inner: Box<dyn InputLogger>) -> Self {
        Self {
            inner,
            muted: false,
        }
    }

    pub(in crate::engine) fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    pub(in crate::engine) fn replace(&mut self, inner: Box<dyn InputLogger>) {
        self.inner = inner;
    }

    pub(in crate::engine) fn inner_mut(&mut self) -> &mut dyn InputLogger {
        self.inner.as_mut()
    }
}

impl InputLogger for MutedLogger {
    fn record(&mut self, entry: InputLogEntry) {
        if !self.muted {
            self.inner.record(entry);
        }
    }

    fn flush(&mut self) {
        self.inner.flush();
    }

    fn is_enabled(&self) -> bool {
        self.inner.is_enabled()
    }
}
