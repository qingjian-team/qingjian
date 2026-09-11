use qingjian_core::CandidateLayout;
use qingjian_platform::protocol::PreeditSegment;

/// 当前组句缓冲对应的展示状态。缓冲变化（敲字 / 退格 / 移光标 / 上屏）时重建一次，
/// 导航（上下 / 翻页）只在它上面挪高亮；云端候选异步到达时并进 `layout`。
pub(super) enum Composed {
    /// 正常查到候选。
    Candidates {
        /// preedit 分段。
        preedit: Vec<PreeditSegment>,

        /// 光标在 marked text 里的字符位置。
        cursor: usize,

        /// 本地 + 云端槽位的候选布局。
        layout: CandidateLayout,
    },

    /// 查询失败（如中途上屏后剩下不可切分的残余）：显示原始拼音、无候选。
    /// 绝不能返回空帧，否则组句非空却没有候选窗，只有 clear / 切输入法能解。
    Raw {
        /// 原始拼音。
        text: String,

        /// 光标字符位置。
        cursor: usize,
    },
}
