//! 词图搜索的一条部分路径。

/// 一条部分路径的末尾节点。
pub(super) struct Node {
    /// 这个词从第几个音节开始。
    pub(super) start: usize,

    /// 词。
    pub(super) text: String,

    /// 到当前位置的文本，剪枝时按文本与上下文去重。
    pub(super) prefix: String,

    /// 词的音节。
    pub(super) syllables: Vec<String>,

    /// 到此为止的累计得分。
    pub(super) score: f64,

    /// 累计得分里静态模型的部分（见 `Conversion::static_score`）。
    pub(super) static_score: f64,

    /// 前驱在 `nodes[start]` 里的下标；`start == 0` 时无意义。
    pub(super) back: usize,

    /// 是占位音节。
    pub(super) placeholder: bool,

    /// 到此为止路径上模糊音 / 敲错变体的代价之和（已从 `score` 里扣掉，另记一份给调用方判断路径是不是原样）。
    pub(super) penalty: f64,
}
