//! 音节 DAG 上的一条边：供 [`super::convert_lattice`] 在带边界歧义的词图上做一次统一 Viterbi。

use qingjian_dictionary::SyllablePattern;

/// 音节 DAG 上的一条边：覆盖一个音节位置，指向下一个顶点的下标。
#[derive(Debug, Clone)]
pub struct LatticeEdge<'a> {
    /// 目标顶点下标（严格大于起点下标）。
    pub to: usize,

    /// 这一音节位置的合法写法。
    pub patterns: Vec<SyllablePattern<'a>>,

    /// 走这条边的额外代价（0 表示无扣分，从路径得分里扣掉）。
    pub penalty: f64,
}
