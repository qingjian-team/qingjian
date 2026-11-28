//! 九宫格字节偏移音节 DAG：保留全部合法音节切法与边界，供词级查找与统一整句 Viterbi 共用。

use crate::parser::MAX_SYLLABLE_LEN;

use super::table::CODE_TABLE;
use super::{LatticeEdge, Position, Shape};

/// 九宫格输入的字节偏移音节 DAG。
///
/// 顶点是输入串的字节偏移 `0..=end`，`edges[start]` 存从 `start` 出发且能走到 `end` 的全部单音节边。
/// 当存在「不靠中间单码简拼、全部由完整音节（或已选定字母与末尾前缀）走到 `end`」的路径时，
/// 提前裁掉中间未选定的单码简拼边，避免长句里把四码完整音节拆成「单码声母 + 三码音节」造成误切与分支膨胀；
/// 纯简拼或含无法完整切分的混合串（`complete_path == false`）则保留全部简拼边。
#[derive(Debug, Clone, Default)]
pub struct Lattice {
    /// 能切到的最远字节偏移（之后是切不动的尾巴）。
    end: usize,

    /// `edges[start]` 是从字节偏移 `start` 出发、且能走到 `end` 的全部音节边。
    edges: Vec<Vec<LatticeEdge>>,

    /// 是否存在不依赖中间未选定简拼即可走到 `end` 的路径。
    complete_path: bool,
}

impl Lattice {
    /// 从输入串构造音节 DAG，返回 `(lattice, tail)`；第一个字符都切不动时 `lattice.is_empty()`。
    pub fn build(input: &str) -> (Self, &str) {
        let bytes = input.as_bytes();
        let n = bytes.len();
        if n == 0 {
            return (Self::default(), input);
        }
        let mut reachable = vec![false; n + 1];
        let mut complete_reachable = vec![false; n + 1];
        reachable[0] = true;
        complete_reachable[0] = true;
        let mut raw_edges: Vec<Vec<LatticeEdge>> = vec![Vec::new(); n + 1];

        for start in 0..n {
            if !reachable[start] || bytes[start] == b'\'' {
                continue;
            }
            let pinned_here = start == 0 || bytes[start - 1] == b'\'';
            let limit = (start + MAX_SYLLABLE_LEN).min(n);
            for raw_end in start + 1..=limit {
                if bytes[raw_end - 1] == b'\'' {
                    break;
                }
                let last = raw_end == n;
                let Some(position) =
                    CODE_TABLE.position(&input[start..raw_end], start, last, pinned_here)
                else {
                    continue;
                };
                let next = skip_separators(bytes, raw_end);
                reachable[next] = true;
                if complete_reachable[start] && (position.complete || position.pinned || last) {
                    complete_reachable[next] = true;
                }
                raw_edges[start].push(LatticeEdge { position, next });
            }
        }

        let end = (1..=n).rev().find(|&pos| reachable[pos]).unwrap_or(0);
        if end == 0 {
            return (Self::default(), input);
        }
        let complete_path = complete_reachable[end];
        let mut edges = raw_edges;
        edges.truncate(end);

        if complete_path {
            for outgoing in &mut edges {
                outgoing.retain(|edge| {
                    edge.position.complete || edge.position.pinned || edge.next == end
                });
            }
        }

        let mut can_reach = vec![false; end + 1];
        can_reach[end] = true;
        for start in (0..end).rev() {
            edges[start].retain(|edge| edge.next <= end && can_reach[edge.next]);
            if !edges[start].is_empty() {
                can_reach[start] = true;
            }
        }

        (
            Self {
                end,
                edges,
                complete_path,
            },
            &input[end..],
        )
    }

    /// 没有任何可切分的边。
    pub fn is_empty(&self) -> bool {
        self.end == 0 || self.edges.first().is_none_or(Vec::is_empty)
    }

    /// 能切到的最远字节偏移。
    pub fn end(&self) -> usize {
        self.end
    }

    /// 是否存在不依赖中间未选定简拼即可走到末尾的完整路径。
    pub fn has_complete_path(&self) -> bool {
        self.complete_path
    }

    /// 全部顶点的出边表，长度为 `end`。
    pub fn edges(&self) -> &[Vec<LatticeEdge>] {
        &self.edges
    }

    /// 字节偏移 `start` 处的出边。
    pub fn edges_at(&self, start: usize) -> &[LatticeEdge] {
        self.edges.get(start).map_or(&[], Vec::as_slice)
    }

    /// 按结构排序键（音节少 > 不完整音节少 > 前面的音节长）从 DAG 里线性动态规划出最优单条形状。
    /// 给拼音列起点、叹词判断、未选定偏好和拼音行兜底用，无需维护多条形状束。
    pub fn best_shape(&self) -> Option<Shape> {
        if self.is_empty() {
            return None;
        }
        let mut best: Vec<Option<Shape>> = vec![None; self.end + 1];
        best[0] = Some(Shape::default());
        for start in 0..self.end {
            let Some(prefix) = best[start].clone() else {
                continue;
            };
            for edge in &self.edges[start] {
                let candidate = prefix.extended(edge.position.clone());
                let replace = best[edge.next]
                    .as_ref()
                    .is_none_or(|known| candidate.sort_key() < known.sort_key());
                if replace {
                    best[edge.next] = Some(candidate);
                }
            }
        }
        best[self.end].take()
    }

    /// 整句是否至少包含两个音节（存在至少一条两步及以上走到 `end` 的路径）。
    pub fn has_multi_syllable_path(&self) -> bool {
        if self.is_empty() {
            return false;
        }
        self.edges[0].iter().any(|edge| edge.next < self.end)
    }

    /// 从任意顶点 `start` 沿 DAG 结构最优走到 `end` 的位置序列，给拼音行补齐后缀用。
    pub fn suffix_positions(&self, from: usize) -> Vec<&Position> {
        if from >= self.end {
            return Vec::new();
        }
        // 后向 DP：音节数少 > 不完整音节少 > 首边长
        let mut best: Vec<Option<(usize, usize, usize, usize)>> = vec![None; self.end + 1];
        best[self.end] = Some((0, 0, 0, self.end));
        for pos in (from..self.end).rev() {
            let mut chosen: Option<(usize, usize, usize, usize)> = None;
            for (edge_idx, edge) in self.edges[pos].iter().enumerate() {
                let Some((count, incomplete, _, _)) = best[edge.next] else {
                    continue;
                };
                let key = (
                    count + 1,
                    incomplete + usize::from(!edge.position.complete),
                    usize::MAX - edge.position.len,
                    edge_idx,
                );
                if chosen.is_none_or(|cur| key < cur) {
                    chosen = Some(key);
                }
            }
            best[pos] = chosen;
        }
        let mut out = Vec::new();
        let mut pos = from;
        while pos < self.end {
            let Some((_, _, _, edge_idx)) = best[pos] else {
                break;
            };
            let edge = &self.edges[pos][edge_idx];
            out.push(&edge.position);
            pos = edge.next;
        }
        out
    }
}

/// 跳过紧跟在某段音节后的全部分隔符 `'`。
fn skip_separators(bytes: &[u8], mut offset: usize) -> usize {
    while offset < bytes.len() && bytes[offset] == b'\'' {
        offset += 1;
    }
    offset
}
