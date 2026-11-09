//! 当前输入会话的 UI 状态：候选排布、高亮、页码、preedit。
//!
//! 放在 Host 里而不是控制器的 ivars 里，是因为联想结果由定时器送达，那时手上没有控制器；
//! 反正 Engine 的缓冲区也是全进程一份，会话状态跟着它走。
//! 候选的分页与云端词的位置由 Core 的 [`CandidateLayout`] 定，这里只管高亮与页码。

use qingjian_core::candidate::arrows::{Focus, Matrix};
use qingjian_core::{Candidate, CandidateLayout, Cell, GRID_ROWS, Grid, Query};

use crate::candidates::Preedit;

#[derive(Debug, Default)]
pub struct Session {
    /// 上一次查询的候选排布（本地候选 + 云端词）。
    pub layout: CandidateLayout,

    /// 高亮的格子下标（在整个排布里的绝对位置）。
    pub highlighted: usize,

    /// 当前页。
    pub page: usize,

    /// 这轮查询里用户用方向键 / 翻页键动过高亮。动过就不再拿重排结果换掉候选。
    pub navigated: bool,

    /// 候选窗口顶部显示的拼音行（分段 + 光标）。
    pub preedit: Option<Preedit>,

    /// 这轮查询的结果（候选已挪进 `layout`）：高亮换了候选时拼音行按它的读音重切，见 [`Query::follow`]。
    pub spelling: Option<Query>,

    /// 横排展开成矩阵时的视口；`None` 是单行。新一轮查询收回单行。
    pub grid: Option<Grid>,

    /// 方向键分两区时（[`qingjian_core::candidate::arrows`]）归拼音行还是候选。改拼音时每敲一键都是新一轮查询，
    /// 所以它跨查询保留，整段输入结束（`preedit` 为空）才回到候选。
    pub focus: Focus,
}

impl Session {
    /// 新一轮查询：候选换掉，选中第一个真实候选。
    pub fn reset(
        &mut self,
        preedit: Option<Preedit>,
        candidates: Vec<Candidate>,
        page_size: usize,
        slots: usize,
    ) {
        self.preedit = preedit;
        self.spelling = None;
        self.layout = CandidateLayout::new(candidates, page_size, slots);
        self.highlighted = (0..self.layout.len())
            .find(|&i| self.layout.candidate(i).is_some())
            .unwrap_or(0);
        self.page = self.highlighted / self.layout.page_size();
        self.navigated = false;
        self.grid = None;
        if self.preedit.is_none() {
            self.focus = Focus::default();
        }
    }

    /// 给 [`qingjian_core::candidate::arrows::route`] 的矩阵状态。`enabled` 是开没开「横排时展开成多行」。
    pub fn matrix(&self, enabled: bool) -> Matrix {
        match self.grid {
            Some(_) => Matrix::Open {
                first_row: self.highlighted < self.layout.page_size(),
            },
            None if enabled => Matrix::Closed,
            None => Matrix::Off,
        }
    }

    /// 矩阵里竖着移 `delta` 行；还是单行就先从当前页展开（展开本身也算变化）。返回要不要重画。
    pub fn move_rows(&mut self, delta: isize) -> bool {
        let expanding = self.grid.is_none();
        let mut grid = self
            .grid
            .unwrap_or_else(|| Grid::at(&self.layout, self.page));
        let moved = grid.move_rows(&self.layout, self.highlighted, delta);
        self.grid = Some(grid);
        self.land(moved) || expanding
    }

    /// 矩阵里整屏翻（翻页键）：一次 [`GRID_ROWS`] 行。
    pub fn move_screens(&mut self, delta: isize) -> bool {
        self.move_rows(delta * GRID_ROWS as isize)
    }

    /// 矩阵里按阅读顺序移一格；没展开时不动。返回要不要重画。
    pub fn move_cells(&mut self, delta: isize) -> bool {
        let Some(mut grid) = self.grid else {
            return false;
        };
        let moved = grid.move_cells(&self.layout, self.highlighted, delta);
        self.grid = Some(grid);
        self.land(moved)
    }

    /// 符号面板换上一个分类：每行 `columns` 格，直接展开成矩阵，高亮在第一格。
    pub fn open_panel(&mut self, items: Vec<Candidate>, columns: usize) {
        self.reset(None, items, columns, 0);
        self.grid = Some(Grid::at(&self.layout, 0));
    }

    /// 收回单行，高亮留在原处。返回原来是不是展开着。
    pub fn collapse(&mut self) -> bool {
        self.grid.take().is_some()
    }

    /// 矩阵里顶在第一排还往上：高亮回到第一个候选、视口滚回顶部。返回要不要重画。
    pub fn jump_to_first(&mut self) -> bool {
        if self.grid.is_none() || self.highlighted == 0 {
            return false;
        }
        self.grid = Some(Grid::at(&self.layout, 0));
        self.highlighted = 0;
        self.page = 0;
        self.navigated = true;
        true
    }

    /// 矩阵视口里的格子（按行优先排开）与每行几格；没展开返回 `None`。
    pub fn grid_cells(&self) -> Option<(Vec<Cell<'_>>, usize)> {
        let grid = self.grid?;
        let columns = self.layout.page_size();
        let mut cells = Vec::new();
        for row in grid.rows(&self.layout) {
            let mut page = self.layout.page(row);
            page.resize(columns, Cell::Empty);
            cells.extend(page);
        }
        Some((cells, columns))
    }

    /// 视口滚 `delta` 行（面板里的滚轮），高亮滚出去时跟进视口。返回要不要重画。
    pub fn scroll_rows(&mut self, delta: isize) -> bool {
        let Some(mut grid) = self.grid else {
            return false;
        };
        let moved = grid.scroll(&self.layout, self.highlighted, delta);
        self.grid = Some(grid);
        self.land(moved)
    }

    /// 把高亮挪到 `index`（点面板格子）。返回动没动。
    pub fn select(&mut self, index: usize) -> bool {
        if index == self.highlighted || self.layout.candidate(index).is_none() {
            return false;
        }
        self.land(Some(index))
    }

    /// 高亮落到 `index`（`None` 是没动），页号跟着走。
    fn land(&mut self, index: Option<usize>) -> bool {
        let Some(index) = index else {
            return false;
        };
        self.highlighted = index;
        self.page = index / self.layout.page_size();
        self.navigated = true;
        true
    }

    /// 第 `index` 格的候选。
    pub fn candidate(&self, index: usize) -> Option<Candidate> {
        self.layout.candidate(index).cloned()
    }

    /// 当前页第 `offset` 格在整个排布里的下标；越界返回 `None`。
    pub fn index_on_page(&self, offset: usize) -> Option<usize> {
        let index = self.page * self.layout.page_size() + offset;
        (offset < self.layout.page_size() && index < self.layout.len()).then_some(index)
    }

    /// 候选窗口里第 `offset` 格在整个排布里的下标：矩阵展开时按视口行优先数（视口是从 `top` 起连续的几页），
    /// 没展开时同 [`Self::index_on_page`]。越界返回 `None`。点击上屏用。
    pub fn index_in_view(&self, offset: usize) -> Option<usize> {
        let Some(grid) = self.grid else {
            return self.index_on_page(offset);
        };
        let index = grid.top() * self.layout.page_size() + offset;
        (index < self.layout.len()).then_some(index)
    }

    /// 当前页的格子。
    pub fn page_cells(&self) -> Vec<Cell<'_>> {
        self.layout.page(self.page)
    }

    /// 移到当前页的第一个或最后一个真实候选，跳过空位。
    pub fn move_to_page_edge(&mut self, last: bool) -> bool {
        let start = self.page * self.layout.page_size();
        let end = (start + self.layout.page_size()).min(self.layout.len());
        let mut indices = start..end;
        let target = if last {
            indices.rfind(|&index| self.layout.candidate(index).is_some())
        } else {
            indices.find(|&index| self.layout.candidate(index).is_some())
        };
        target.is_some_and(|index| self.select(index))
    }

    /// 高亮上下移动，越过页边自动翻页。返回是否有变化。
    pub fn move_highlight(&mut self, delta: isize) -> bool {
        let len = self.layout.len();
        if len == 0 {
            return false;
        }
        let current = self.highlighted as isize;
        let mut next = (current + delta).clamp(0, len as isize - 1) as usize;
        while self.layout.candidate(next).is_none() {
            let candidate = next as isize + delta.signum();
            if candidate < 0 || candidate >= len as isize || delta == 0 {
                return false;
            }
            next = candidate as usize;
        }
        if next == self.highlighted {
            return false;
        }
        self.highlighted = next;
        self.page = next / self.layout.page_size();
        self.navigated = true;
        true
    }

    /// 往 `delta` 方向翻一页该落到哪个候选下标（跳过没有候选的页）；翻不动（没有上一页 / 下一页）返回 `None`。
    fn page_target(&self, delta: isize) -> Option<usize> {
        let pages = self.layout.pages().max(1);
        let page_size = self.layout.page_size();
        let mut next = (self.page as isize + delta).clamp(0, pages as isize - 1) as usize;
        loop {
            if next == self.page {
                return None;
            }
            let start = next * page_size;
            if let Some(index) = (start..(start + page_size).min(self.layout.len()))
                .find(|&i| self.layout.candidate(i).is_some())
            {
                return Some(index);
            }
            let following = next as isize + delta.signum();
            if following < 0 || following >= pages as isize || delta == 0 {
                return None;
            }
            next = following as usize;
        }
    }

    /// 这个方向还翻得动吗（不改状态地问一句）。翻页键「翻不动就落字」用，见
    /// `docs/user/personalize/keys-settings.md` 的「翻页键」。
    pub fn can_turn_page(&self, delta: isize) -> bool {
        self.page_target(delta).is_some()
    }

    /// 翻页并选中新页第一个真实候选，跳过没有候选的页。
    pub fn turn_page(&mut self, delta: isize) -> bool {
        let Some(index) = self.page_target(delta) else {
            return false;
        };
        self.page = index / self.layout.page_size();
        self.highlighted = index;
        self.navigated = true;
        true
    }

    pub fn pages(&self) -> usize {
        self.layout.pages()
    }
}

#[cfg(test)]
mod tests;
