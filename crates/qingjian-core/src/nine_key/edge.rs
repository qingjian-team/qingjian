//! 九宫格音节 DAG 里的一条边：覆盖一个位置并指向下一个音节的字节起点。

use super::Position;

/// 切分 DAG 里的一条音节边：覆盖一个位置并指向下一个音节的字节起点（已跳过紧跟的 `'`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatticeEdge {
    /// 这一段的码、长度与全部合法写法。
    pub position: Position,

    /// 走完这条边后落到的字节偏移（已跳过紧跟的 `'`）。
    pub next: usize,
}
