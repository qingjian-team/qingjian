//! 组句展示状态的枚举。

use qingjian_core::{CandidateLayout, Query};
use qingjian_platform::protocol::PreeditSegment;

/// 当前组句缓冲对应的展示状态。缓冲变化时重建，导航只挪高亮，云端词异步并进 `layout`。
pub(crate) enum Composed {
    /// 正常查到候选。
    Candidates {
        /// preedit 分段。
        preedit: Vec<PreeditSegment>,

        /// 光标在 marked text 里的字符位置。
        cursor: usize,

        /// 本地 + 云端槽位的候选布局。
        layout: CandidateLayout,

        /// 这轮查询的结果（候选已挪进 `layout`）：高亮候选的读音切法不同时拼音行按它重切，见 [`Query::follow`]。
        spelling: Box<Query>,
    },

    /// 查询失败（如上屏后剩下不可切分的残余）：显示原始拼音、无候选。
    /// 不能回空帧，否则组句非空却没有候选窗，只有 clear / 切输入法能解。
    Raw {
        /// 原始拼音。
        text: String,

        /// 光标字符位置。
        cursor: usize,
    },
}
