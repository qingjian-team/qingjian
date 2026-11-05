//! 组句中挪高亮、翻页、横排矩阵的行列移动。

use qingjian_core::candidate::arrows::Matrix;
use qingjian_core::{GRID_ROWS, Grid};

use super::Composed;
use crate::dispatch::Router;

impl Router {
    /// 高亮移动 `delta`，夹在 `[0, 末尾]`，到页边自然换页。
    pub(in crate::dispatch) fn move_highlight(&mut self, delta: isize) {
        let count = self.candidate_count();
        if count == 0 {
            self.highlight = 0;
            return;
        }
        let next = (self.highlight as isize + delta).clamp(0, count as isize - 1) as usize;
        self.navigated |= next != self.highlight;
        self.highlight = next;
    }

    /// 整页翻 `step`，高亮落到目标页第一个候选；矩阵展开着时一次翻一屏（与 macOS 壳的 `turn_page` 一致）。
    /// 开了「翻页键展开」（`RouterConfig::page_keys_expand`）时单行先展开，高亮移到上 / 下一行，与 `↓` 展开同一套。
    pub(in crate::dispatch) fn page(&mut self, step: isize) {
        let moved = if self.grid.is_some() {
            Some(self.move_screens(step))
        } else if self.config.page_keys_expand() {
            Some(self.shift_grid_rows(step))
        } else {
            None
        };
        if let Some(moved) = moved {
            if moved {
                self.engine.note_page_turn();
            }
            return;
        }
        let count = self.candidate_count();
        if count == 0 {
            self.highlight = 0;
            return;
        }
        let page_size = self.config.page_size;
        let page_count = count.div_ceil(page_size);
        let current = (self.highlight / page_size) as isize;
        let target = (current + step).clamp(0, page_count as isize - 1) as usize;
        if target != current as usize {
            self.navigated = true;
            self.engine.note_page_turn();
        }
        self.highlight = (target * page_size).min(count - 1);
    }

    /// 往 `step` 方向还有没有页可翻。
    pub(in crate::dispatch) fn can_page(&self, step: isize) -> bool {
        let page_count = self.candidate_count().div_ceil(self.config.page_size) as isize;
        let target = (self.highlight / self.config.page_size) as isize + step;
        (0..page_count).contains(&target)
    }

    /// 横排矩阵：上 / 下键竖着移 `delta` 行（列不变，目标格没候选就取最近的）；还是单行就先从当前页展开
    /// （展开本身算变化）。返回高亮是否真的动了。
    pub(in crate::dispatch) fn move_rows(&mut self, delta: isize) -> bool {
        let expanding = self.grid.is_none();
        self.shift_grid_rows(delta) || expanding
    }

    /// 矩阵里顶在第一排还往上：高亮回到第一个候选、视口滚回顶部。返回是否动了。
    pub(in crate::dispatch) fn jump_to_first(&mut self) -> bool {
        let Some(Composed::Candidates { layout, .. }) = &self.composed else {
            return false;
        };
        if self.grid.is_none() || self.highlight == 0 {
            return false;
        }
        self.highlight = 0;
        self.grid = Some(Grid::at(layout, 0));
        self.navigated = true;
        true
    }

    /// 方向键按矩阵那套走：开了「横排时 ↑ / ↓ 展开」，或者矩阵已经展开着（翻页键展开的也算，不然展开后方向键会按单行移、走出视口）。
    pub(in crate::dispatch) fn grid_keys(&self) -> bool {
        self.config.grid_keys() || self.grid.is_some()
    }

    /// 给 `arrows::route` 的矩阵状态：没开「横排时展开成多行」、单行、展开后高亮在不在第一行。
    pub(in crate::dispatch) fn matrix(&self) -> Matrix {
        match self.grid {
            Some(_) => Matrix::Open {
                first_row: self.highlight < self.config.page_size,
            },
            None if self.config.grid_keys() => Matrix::Closed,
            None => Matrix::Off,
        }
    }

    /// 矩阵里按阅读顺序移一格（越过行尾到下一行开头），跳过空位；没展开时不动。返回是否动了。
    pub(in crate::dispatch) fn move_cells(&mut self, delta: isize) -> bool {
        let Some(Composed::Candidates { layout, .. }) = &self.composed else {
            return false;
        };
        let Some(mut grid) = self.grid else {
            return false;
        };
        let moved = grid.move_cells(layout, self.highlight, delta);
        self.grid = Some(grid);
        self.land(moved)
    }

    /// 矩阵里整屏翻（翻页键）：一次 [`GRID_ROWS`] 行。返回是否动了。
    pub(in crate::dispatch) fn move_screens(&mut self, delta: isize) -> bool {
        self.shift_grid_rows(delta * GRID_ROWS as isize)
    }

    /// 收回单行（Esc 第一下），高亮留在原处。返回原来是不是展开着。
    pub(in crate::dispatch) fn collapse_grid(&mut self) -> bool {
        self.grid.take().is_some()
    }

    /// 矩阵视口里竖着移 `delta` 行；没展开就从当前页展开。返回高亮是否真的动了。
    fn shift_grid_rows(&mut self, delta: isize) -> bool {
        let Some(Composed::Candidates { layout, .. }) = &self.composed else {
            return false;
        };
        let page = self.highlight / self.config.page_size;
        let mut grid = self.grid.take().unwrap_or_else(|| Grid::at(layout, page));
        let moved = grid.move_rows(layout, self.highlight, delta);
        self.grid = Some(grid);
        self.land(moved)
    }

    /// 高亮落到 `index`（`None` 是没动）。返回是否动了。
    fn land(&mut self, index: Option<usize>) -> bool {
        let Some(index) = index else {
            return false;
        };
        self.highlight = index;
        self.navigated = true;
        true
    }
}
