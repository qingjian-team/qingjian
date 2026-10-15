//! 组句展示状态的枚举。

use qingjian_core::CandidateLayout;
use qingjian_platform::protocol::PreeditSegment;

use super::SentenceTranslation;

/// 当前组句缓冲对应的展示状态。缓冲变化时重建，导航只挪高亮，云端词异步并进 `layout`。
pub(crate) enum Composed {
    /// 正常查到候选。
    Candidates {
        /// preedit 分段。
        preedit: Vec<PreeditSegment>,

        /// 光标在拼音行里的字符位置。
        cursor: usize,

        /// 双拼「输入框显示原始按键」开着时，应用输入框里改放这一串；关着为 `None`，输入框与拼音行一样。
        typed_keys: Option<TypedKeys>,

        /// 本地 + 云端槽位的候选布局。
        layout: CandidateLayout,

        /// 首条本地整句候选的云端译文；未返回时为 `None`。
        sentence_translation: Option<SentenceTranslation>,
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

/// 应用输入框里的原始按键与光标（字符下标）。
pub(crate) struct TypedKeys {
    pub(crate) text: String,
    pub(crate) cursor: usize,
}
