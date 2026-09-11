//! 云服务页「测试连接」的状态，挂在 [`Settings`](super::Settings) 上，由根组件的 `update` 推进、云服务页显示。

/// 「测试连接」的状态。
#[derive(Clone)]
pub(crate) enum CloudStatus {
    /// 没测过。
    Idle,

    /// 正在测。
    Testing,

    /// 成功（带说明）。
    Ok(String),

    /// 失败（带错误）。
    Failed(String),
}
