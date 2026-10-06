//! 横排展开成矩阵时的视口与高亮移动：一行就是一页候选，固定显示几行，高亮移出视口时按行滚动。
//! 与分页一样是展示规则，各平台壳共用，所以放在 Core；壳只转发方向键。

use std::ops::Range;

use super::CandidateLayout;

/// 矩阵视口显示几行。
pub const GRID_ROWS: usize = 6;

/// 一格最多按几个字宽留：四字词能完整显示，再长的候选在格子里截断。
pub const MAX_CELL_EMS: f32 = 4.0;

/// 估宽时非宽字符（拉丁字母、数字、半角标点）按几个字宽算。
const NARROW_EMS: f32 = 0.62;

/// 云端词前面的云朵图标按几个字宽算。
const CLOUD_EMS: f32 = 1.1;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Grid {
    /// 视口第一行的行号（页号）。
    top: usize,
}

impl Grid {
    /// 从第 `row` 行展开：它当视口第一行；后面不够一屏时视口往上补齐，窗口高度才不会因为在末几页展开而变矮。
    pub fn at(layout: &CandidateLayout, row: usize) -> Self {
        Self {
            top: row.min(layout.pages().saturating_sub(GRID_ROWS)),
        }
    }

    /// 各列要留几个字宽：按**整份**候选估（不只是视口里那几行），同一轮查询里滚动、移动高亮时列宽不变，窗口不跳。
    /// 按字符数估而不实测——汉字一个字宽、拉丁字母不到一个——几百条候选每帧实测太慢；上限 [`MAX_CELL_EMS`]。
    pub fn column_ems(layout: &CandidateLayout) -> Vec<f32> {
        let columns = layout.page_size();
        let mut widths = vec![0.0_f32; columns];
        for (i, cell) in layout.cells().iter().enumerate() {
            let Some(candidate) = cell.candidate() else {
                continue;
            };
            let mut ems: f32 = candidate
                .text
                .chars()
                .map(|c| if c.is_ascii() { NARROW_EMS } else { 1.0 })
                .sum();
            if matches!(cell, super::Cell::Cloud(_)) {
                ems += CLOUD_EMS;
            }
            let width = &mut widths[i % columns];
            *width = width.max(ems.min(MAX_CELL_EMS));
        }
        widths
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
    use super::{GRID_ROWS, Grid, MAX_CELL_EMS};
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
        let mut grid = Grid::at(&layout, 0);
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
        let mut grid = Grid::at(&layout, 0);
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
    fn expanding_near_the_end_fills_the_viewport_upwards() {
        let layout = layout(100, 5);
        assert_eq!(Grid::at(&layout, 3).rows(&layout), 3..9);
        assert_eq!(Grid::at(&layout, 19).rows(&layout), 14..20);
        // 总共不到一屏：从头显示
        let short = super::super::CandidateLayout::new(Vec::new(), 5, 0);
        assert_eq!(Grid::at(&short, 0).top(), 0);
    }

    #[test]
    fn column_widths_come_from_the_whole_list_and_are_capped() {
        let mut candidates: Vec<Candidate> = ["是", "时候", "abc", "一心一意", "是不是因为我们"]
            .iter()
            .map(|text| Candidate {
                text: (*text).into(),
                kind: CandidateKind::Chinese,
                syllables: vec!["a".into()],
                reading: None,
                translation: None,
                aux_code: None,
            })
            .collect();
        // 第二行第一列有个三字词：哪怕不在视口里，第一列也按它留宽
        candidates.push(Candidate {
            text: "事实上".into(),
            ..candidates[0].clone()
        });
        let layout = CandidateLayout::new(candidates, 5, 0);
        let ems = Grid::column_ems(&layout);
        assert_eq!(ems.len(), 5);
        assert_eq!(ems[0], 3.0);
        assert_eq!(ems[1], 2.0);
        assert!((ems[2] - 1.86).abs() < 1e-5);
        assert_eq!(ems[3], 4.0);
        assert_eq!(ems[4], MAX_CELL_EMS);
    }

    #[test]
    fn cells_move_in_reading_order_across_rows() {
        let layout = layout(7, 5);
        let mut grid = Grid::at(&layout, 0);
        assert_eq!(grid.move_cells(&layout, 4, 1), Some(5));
        assert_eq!(grid.move_cells(&layout, 5, -1), Some(4));
        assert_eq!(grid.move_cells(&layout, 6, 1), None);
        assert_eq!(grid.move_cells(&layout, 0, -1), None);
    }
}
