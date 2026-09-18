//! 「辅码」页触发键录制的状态，挂在 [`Settings`](super::Settings) 上：点「录制」后等用户按一个键。

/// 触发键录制状态。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Recorder {
    /// 没在录，控件显示当前触发键。
    #[default]
    Idle,

    /// 在等用户按下一个键。
    Waiting {
        /// 第几轮等待，从 1 起：录制框拿它当 `KeyedView` 标识，一轮一代新框——
        /// 上一轮敲进来的字符不会被 Reactor 重写回声明值，只能靠重建清掉。
        attempt: u64,
    },
}

impl Recorder {
    /// 进入下一轮等待（录制框跟着换代）。
    pub(crate) fn waiting(self) -> Self {
        let attempt = match self {
            Self::Idle => 0,
            Self::Waiting { attempt } => attempt,
        };
        Self::Waiting {
            attempt: attempt + 1,
        }
    }
}
