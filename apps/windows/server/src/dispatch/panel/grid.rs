//! 面板里当前分类的格子：条目按每行几格排开，视口固定几行，高亮跟着方向键、滚轮与点击走。

use qingjian_core::{Candidate, CandidateLayout, Cell, Grid};

use super::PanelCell;

pub(in crate::dispatch) struct PanelGrid {
    /// 一页就是一行。
    layout: CandidateLayout,

    /// 高亮的条目在整个分类里的下标。
    highlight: usize,

    /// 视口。
    grid: Grid,
}

impl PanelGrid {
    pub(in crate::dispatch) fn new(items: Vec<Candidate>, columns: usize) -> Self {
        let layout = CandidateLayout::new(items, columns.max(1), 0);
        let grid = Grid::at(&layout, 0);
        Self {
            layout,
            highlight: 0,
            grid,
        }
    }

    pub(in crate::dispatch) fn columns(&self) -> usize {
        self.layout.page_size()
    }

    pub(in crate::dispatch) fn highlight(&self) -> usize {
        self.highlight
    }

    pub(in crate::dispatch) fn candidate(&self, index: usize) -> Option<&Candidate> {
        self.layout.candidate(index)
    }

    /// 竖着移 `delta` 行。返回动没动。
    pub(in crate::dispatch) fn move_rows(&mut self, delta: isize) -> bool {
        let moved = self.grid.move_rows(&self.layout, self.highlight, delta);
        self.land(moved)
    }

    /// 按阅读顺序移一格。
    pub(in crate::dispatch) fn move_cells(&mut self, delta: isize) -> bool {
        let moved = self.grid.move_cells(&self.layout, self.highlight, delta);
        self.land(moved)
    }

    /// 视口滚 `delta` 行，高亮滚出去时跟进视口。返回要不要重画。
    pub(in crate::dispatch) fn scroll(&mut self, delta: isize) -> bool {
        let moved = self.grid.scroll(&self.layout, self.highlight, delta);
        self.land(moved)
    }

    /// 高亮挪到 `index`（点格子）。返回动没动。
    pub(in crate::dispatch) fn select(&mut self, index: usize) -> bool {
        if index == self.highlight || self.layout.candidate(index).is_none() {
            return false;
        }
        self.land(Some(index))
    }

    fn land(&mut self, index: Option<usize>) -> bool {
        let Some(index) = index else {
            return false;
        };
        self.highlight = index;
        true
    }

    /// 视口里第 `offset` 格（行优先）在整个分类里的下标；空格子返回 `None`。
    pub(in crate::dispatch) fn index_in_view(&self, offset: usize) -> Option<usize> {
        let index = self.grid.top() * self.columns() + offset;
        self.layout.candidate(index).map(|_| index)
    }

    /// 高亮那一行的第 `digit` 格（数字键，从 1 起）。
    pub(in crate::dispatch) fn index_in_row(&self, digit: usize) -> Option<usize> {
        let columns = self.columns();
        let index = self.highlight / columns * columns + digit.checked_sub(1)?;
        (digit <= columns && self.layout.candidate(index).is_some()).then_some(index)
    }

    /// 视口里的格子，按行优先排开，行尾不满的补空。
    pub(in crate::dispatch) fn cells(&self) -> Vec<Option<PanelCell>> {
        let columns = self.columns();
        let mut cells = Vec::new();
        for row in self.grid.rows(&self.layout) {
            let mut page = self.layout.page(row);
            page.resize(columns, Cell::Empty);
            cells.extend(page.iter().map(|cell| {
                cell.candidate().map(|candidate| PanelCell {
                    text: candidate.text.clone(),
                    name: candidate.reading.clone(),
                })
            }));
        }
        cells
    }

    /// 高亮在视口里是第几格。
    pub(in crate::dispatch) fn highlight_in_view(&self) -> usize {
        self.highlight
            .saturating_sub(self.grid.top() * self.columns())
    }

    /// 不止一行时的「第几行 / 共几行」。
    pub(in crate::dispatch) fn footer(&self) -> Option<String> {
        let rows = self.layout.pages();
        (rows > 1).then(|| format!("{}/{rows}", self.highlight / self.columns() + 1))
    }
}
