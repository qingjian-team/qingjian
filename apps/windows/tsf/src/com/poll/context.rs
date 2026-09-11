//! 轮询定时器每一拍回调要用的东西：按消息窗口句柄从线程表里查出来。

use std::cell::Cell;
use std::rc::Rc;

use crate::com::composition::Shared;
use crate::com::service::SharedClient;

pub(super) struct PollContext {
    /// 引擎层：拉云结果 / 问切模式请求。
    pub(super) engine: SharedClient,

    /// 组句状态：在不在组句 / 翻译评审 / 前台。
    pub(super) shared: Rc<Shared>,

    /// 拍数计数，给 [`MODE_SYNC_EVERY`](super::MODE_SYNC_EVERY) 取模。
    pub(super) ticks: Cell<u32>,
}
