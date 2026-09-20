//! 横排展开成矩阵时的视口与高亮移动：一行就是一页候选，固定显示几行，高亮移出视口时按行滚动。
//! 与分页一样是展示规则，各平台壳共用，所以放在 Core；壳只转发方向键。

use std::ops::Range;

use super::CandidateLayout;

/// 矩阵视口显示几行。
pub const GRID_ROWS: usize = 6;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Grid {
    /// 视口第一行的行号（页号）。
    top: usize,
}

impl Grid {
    /// 从第 `row` 行展开：它先当视口第一行。
    pub fn at(row: usize) -> Self {
        Self { top: row }
    }

    pub fn top(&self) -> usize {
        self.top
    }

    /// 视口里的行号，不超过排布的总行数。
    pub fn rows(&self, layout: &CandidateLayout) -> Range<usize> {
        let end = (self.top + GRID_ROWS).min(layout.pages().max(1));
        self.top.min(end)..end
    }

    /// 高亮竖着移 `delta` 行、列不变；目标格没有候选（行尾不满、固定位置留的空位）就取那一行里离它最近的候选，
    /// 整行都没有就顺着方向再找下一行。返回新的高亮下标，动不了返回 `None`。视口跟着滚。
    pub fn move_rows(
        &mut self,
        layout: &CandidateLayout,
        highlighted: usize,
        delta: isize,
    ) -> Option<usize> {
        let columns = layout.page_size();
        let rows = layout.pages();
        if rows == 0 || delta == 0 {
            return None;
        }
        let (row, column) = (highlighted / columns, highlighted % columns);
        let mut target = (row as isize + delta).clamp(0, rows as isize - 1) as usize;
        while target != row {
            if let Some(index) = nearest_in_row(layout, target, column) {
                self.reveal(index / columns);
                return Some(index);
            }
            let next = target as isize + delta.signum();
            if next < 0 || next >= rows as isize {
                break;
            }
            target = next as usize;
        }
        None
    }

    /// 高亮按阅读顺序移一格（越过行尾到下一行开头），跳过空位。返回新的高亮下标，到头了返回 `None`。
    pub fn move_cells(
        &mut self,
        layout: &CandidateLayout,
        highlighted: usize,
        delta: isize,
    ) -> Option<usize> {
        let step = delta.signum();
        if step == 0 {
            return None;
        }
        let mut index = highlighted as isize + step;
        while index >= 0 && (index as usize) < layout.len() {
            if layout.candidate(index as usize).is_some() {
                self.reveal(index as usize / layout.page_size());
                return Some(index as usize);
            }
            index += step;
        }
        None
    }

    /// 滚动视口让第 `row` 行可见：往上露出就让它当第一行，往下露出就让它当最后一行。
    fn reveal(&mut self, row: usize) {
        if row < self.top {
            self.top = row;
        } else if row >= self.top + GRID_ROWS {
            self.top = row + 1 - GRID_ROWS;
        }
    }
}

/// 第 `row` 行里离第 `column` 列最近的候选下标（同距离取左边的）。
fn nearest_in_row(layout: &CandidateLayout, row: usize, column: usize) -> Option<usize> {
    let columns = layout.page_size();
    let start = row * columns;
    (0..columns)
        .filter(|&c| layout.candidate(start + c).is_some())
        .min_by_key(|&c| (c.abs_diff(column), c))
        .map(|c| start + c)
}

#[cfg(test)]
mod tests {
    use super::{GRID_ROWS, Grid};
    use crate::candidate::{Candidate, CandidateKind, CandidateLayout};

    fn layout(count: usize, page_size: usize) -> CandidateLayout {
        let candidates = (0..count)
            .map(|i| Candidate {
                text: format!("本{i}"),
                kind: CandidateKind::Chinese,
                syllables: vec!["a".into()],
                reading: None,
                translation: None,
                aux_code: None,
            })
            .collect();
        CandidateLayout::new(candidates, page_size, 0)
    }

    #[test]
    fn rows_keep_the_column_and_clamp_to_the_last_candidate_of_a_short_row() {
        let layout = layout(13, 5);
        let mut grid = Grid::at(0);
        // 第 0 行第 3 列 → 第 1 行第 3 列
        assert_eq!(grid.move_rows(&layout, 3, 1), Some(8));
        // 最后一行只有 3 个：第 3 列没有，落到最近的第 2 列
        assert_eq!(grid.move_rows(&layout, 8, 1), Some(12));
        // 到底了再往下不动；往上回到同一列
        assert_eq!(grid.move_rows(&layout, 12, 1), None);
        assert_eq!(grid.move_rows(&layout, 12, -1), Some(7));
        assert_eq!(grid.move_rows(&layout, 2, -1), None);
    }

    #[test]
    fn viewport_scrolls_by_one_row_when_the_highlight_leaves_it() {
        let layout = layout(100, 5);
        let mut grid = Grid::at(0);
        let mut highlighted = 0;
        for _ in 0..GRID_ROWS - 1 {
            highlighted = grid.move_rows(&layout, highlighted, 1).unwrap();
        }
        assert_eq!((highlighted / 5, grid.top()), (5, 0));
        assert_eq!(grid.rows(&layout), 0..6);
        // 再往下一行：视口滚一行
        highlighted = grid.move_rows(&layout, highlighted, 1).unwrap();
        assert_eq!((highlighted / 5, grid.top()), (6, 1));
        // 往上回到视口顶再往上：视口跟着往上滚
        for _ in 0..6 {
            highlighted = grid.move_rows(&layout, highlighted, -1).unwrap();
        }
        assert_eq!((highlighted / 5, grid.top()), (0, 0));
        // 整屏翻（翻页键）：一次 6 行，末尾夹住
        highlighted = grid
            .move_rows(&layout, highlighted, GRID_ROWS as isize)
            .unwrap();
        assert_eq!((highlighted / 5, grid.top()), (6, 1));
        assert_eq!(grid.move_rows(&layout, highlighted, 1000), Some(95));
        assert_eq!(grid.rows(&layout), 14..20);
    }

    #[test]
    fn cells_move_in_reading_order_across_rows() {
        let layout = layout(7, 5);
        let mut grid = Grid::at(0);
        assert_eq!(grid.move_cells(&layout, 4, 1), Some(5));
        assert_eq!(grid.move_cells(&layout, 5, -1), Some(4));
        assert_eq!(grid.move_cells(&layout, 6, 1), None);
        assert_eq!(grid.move_cells(&layout, 0, -1), None);
    }
}
