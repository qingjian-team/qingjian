//! 服务现在的状态：后台线程写，客户端查或等。

use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use super::error::LocalServerError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum State {
    /// 正在起、正在加载模型，或在等端口上旧的服务让出来。
    Starting,

    /// 能发请求了。
    Ready,

    /// 起不来或退出了，后台线程隔一阵会再试。
    Failed(LocalServerError),
}

pub(crate) struct Status {
    state: Mutex<State>,

    /// 状态一变就通知等着的客户端。
    changed: Condvar,
}

impl Status {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State::Starting),
            changed: Condvar::new(),
        }
    }

    pub fn set(&self, state: State) {
        let mut current = self.lock();
        if *current != state {
            *current = state;
            self.changed.notify_all();
        }
    }

    pub fn get(&self) -> State {
        self.lock().clone()
    }

    /// 还在起就等着，最多等 `timeout`，返回那时的状态。
    pub fn wait(&self, timeout: Duration) -> State {
        let (state, _) = self
            .changed
            .wait_timeout_while(self.lock(), timeout, |state| *state == State::Starting)
            .unwrap_or_else(PoisonError::into_inner);
        state.clone()
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
