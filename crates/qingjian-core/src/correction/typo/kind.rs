/// 一个音节里的一处敲错是哪一类，决定它在词图里的代价。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypoKind {
    /// 相邻两键敲反了（`shou` → `shuo`）。
    Transpose,

    /// 敲到了旁边的键（`ni` → `mi`，n 与 m 相邻）。不相邻的键不算敲错（那是读音问题，模糊音管）。
    Substitute,

    /// 多敲了一个键（`gang` → `gan`）。
    Extra,

    /// 少敲了一个键（`gan` → `guan`）。
    Missing,
}

impl TypoKind {
    /// 这类敲错的代价（log 概率的扣分），叠在词的得分上：候选拼音与敲的不同时，要比原样的解释好这么多才排得过。
    ///
    /// 换位与相邻键最常见、代价最低；多敲少敲稍贵。数值按「一个音节敲错的先验约 1%，再分摊到它的十几个变体上」定，
    /// 与整段一处编辑的 `CORRECTION_PENALTY`（5.0）同一量级；定太低时常用词会借着个人词频从敲错边挤掉用户真要的生僻词。
    pub fn cost(self) -> f64 {
        match self {
            Self::Transpose | Self::Substitute => 5.0,
            Self::Extra | Self::Missing => 5.5,
        }
    }
}
