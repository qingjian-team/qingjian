/// 联想策略：观察窗口、条数上限、哪些联想开着。由 [`super::Predictor`] 提供，Engine 据此裁剪上下文、决定发不发。
///
/// 无论壳传来多长的文本，Core 都只发这么多：这是隐私上限的最后一道闸。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PredictionPolicy {
    /// 光标前最多看几个字符。
    pub before: usize,

    /// 光标后最多看几个字符。
    pub after: usize,

    /// 云端词最多补进候选窗口第一页末尾几格；0 表示不要云端词（整句补全照发）。
    pub slots: usize,

    /// 向云端最多要几条（比槽位多要几条，与本地候选重复的去掉后还能填满）。
    pub max_items: usize,

    /// 组句中要不要整句补全。
    pub sentence: bool,
}

impl Default for PredictionPolicy {
    fn default() -> Self {
        Self {
            before: 64,
            after: 32,
            slots: 2,
            max_items: 4,
            sentence: true,
        }
    }
}
