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
    /// 这类敲错的缺省代价（[`TypoCosts::DEFAULT`]）：变体表里同一写法来自几类敲错时按它留代价最低的一类。
    /// 词图打分用引擎当前的 [`TypoCosts`]，不用这个。
    ///
    /// [`TypoCosts`]: super::super::TypoCosts
    /// [`TypoCosts::DEFAULT`]: super::super::TypoCosts::DEFAULT
    pub fn cost(self) -> f64 {
        super::super::TypoCosts::DEFAULT.cost(self)
    }
}
