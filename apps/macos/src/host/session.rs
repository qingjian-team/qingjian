//! 当前输入会话的 UI 状态：候选排布、高亮、页码、preedit。
//!
//! 放在 Host 里而不是控制器的 ivars 里，是因为联想结果由定时器送达，那时手上没有控制器；
//! 反正 Engine 的缓冲区也是全进程一份，会话状态跟着它走。
//! 候选的分页与云端词的位置由 Core 的 [`CandidateLayout`] 定，这里只管高亮与页码。

use qingjian_core::{Candidate, CandidateLayout, Cell, GRID_ROWS, Grid};

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

    /// 横排展开成矩阵时的视口；`None` 是单行。新一轮查询收回单行。
    pub grid: Option<Grid>,
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
        self.layout = CandidateLayout::new(candidates, page_size, slots);
        self.highlighted = (0..self.layout.len())
            .find(|&i| self.layout.candidate(i).is_some())
            .unwrap_or(0);
        self.page = self.highlighted / self.layout.page_size();
        self.navigated = false;
        self.grid = None;
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

    /// 矩阵里按阅读顺序移一格；没展开时不动。
    pub fn move_cells(&mut self, delta: isize) -> bool {
        let Some(mut grid) = self.grid else {
            return false;
        };
        let moved = grid.move_cells(&self.layout, self.highlighted, delta);
        self.grid = Some(grid);
        self.land(moved)
    }

    /// 收回单行，高亮留在原处。返回原来是不是展开着。
    pub fn collapse(&mut self) -> bool {
        self.grid.take().is_some()
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

    /// 当前页的格子。
    pub fn page_cells(&self) -> Vec<Cell<'_>> {
        self.layout.page(self.page)
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

    /// 翻页并选中新页第一个真实候选，跳过没有候选的页。
    pub fn turn_page(&mut self, delta: isize) -> bool {
        let pages = self.layout.pages().max(1);
        let current = self.page as isize;
        let mut next = (current + delta).clamp(0, pages as isize - 1) as usize;
        loop {
            if next == self.page {
                return false;
            }
            let start = next * self.layout.page_size();
            if let Some(index) = (start..(start + self.layout.page_size()).min(self.layout.len()))
                .find(|&i| self.layout.candidate(i).is_some())
            {
                self.page = next;
                self.highlighted = index;
                break;
            }
            let following = next as isize + delta.signum();
            if following < 0 || following >= pages as isize || delta == 0 {
                return false;
            }
            next = following as usize;
        }
        self.navigated = true;
        true
    }

    pub fn pages(&self) -> usize {
        self.layout.pages()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qingjian_core::CandidateKind;

    fn candidates(count: usize) -> Vec<Candidate> {
        (0..count)
            .map(|i| Candidate {
                text: format!("本{i}"),
                kind: CandidateKind::Chinese,
                syllables: vec!["a".into()],
                reading: None,
                translation: None,
                aux_code: None,
            })
            .collect()
    }

    #[test]
    fn vertical_keys_expand_the_row_into_a_scrolling_grid() {
        let mut session = Session::default();
        session.reset(None, candidates(100), 5, 0);
        assert!(session.grid_cells().is_none());
        // 左右键在单行时不归候选管
        assert!(!session.move_cells(1));
        // 第一下往下：展开并换到第二行同一列，数字键跟着选第二行
        assert!(session.move_rows(1));
        assert_eq!((session.highlighted, session.page), (5, 1));
        assert_eq!(session.index_on_page(2), Some(7));
        let (cells, columns) = session.grid_cells().unwrap();
        assert_eq!((cells.len(), columns), (30, 5));
        // 展开后左右键按阅读顺序移动，越过行尾到下一行
        assert!(session.move_cells(-1));
        assert_eq!((session.highlighted, session.page), (4, 0));
        // 一直往下：视口按行滚动
        for _ in 0..7 {
            session.move_rows(1);
        }
        assert_eq!(session.page, 7);
        assert_eq!(session.grid.unwrap().top(), 2);
        // 翻页键一次一屏；Esc 收回单行，高亮不动
        assert!(session.move_screens(1));
        assert_eq!(session.page, 13);
        assert!(session.collapse());
        assert!(!session.collapse());
        assert_eq!(session.page, 13);
        // 新一轮查询回到单行
        session.move_rows(1);
        session.reset(None, candidates(3), 5, 0);
        assert!(session.grid.is_none());
        // 只有一行时往上：展开但不动，仍要重画
        assert!(session.move_rows(-1));
        let (cells, _) = session.grid_cells().unwrap();
        assert_eq!(cells.len(), 5);
    }

    #[test]
    fn digits_map_to_cells_on_the_current_page() {
        let mut session = Session::default();
        session.reset(None, candidates(3), 9, 2);
        assert_eq!(session.index_on_page(2), Some(2));
        assert_eq!(session.index_on_page(3), None);
        assert!(session.move_highlight(1));
        assert!(session.move_highlight(5));
        assert_eq!(session.highlighted, 2);
        assert!(!session.move_highlight(1));
    }

    #[test]
    fn paging_follows_the_layout() {
        let mut session = Session::default();
        session.reset(None, candidates(12), 9, 2);
        assert_eq!(session.pages(), 2);
        assert!(session.turn_page(1));
        assert_eq!(session.highlighted, 9);
        assert_eq!(session.index_on_page(0), Some(9));
        assert!(!session.turn_page(1));
    }

    #[test]
    fn navigation_is_remembered_until_the_next_query() {
        let mut session = Session::default();
        session.reset(None, candidates(12), 9, 2);
        assert!(!session.navigated);
        // 顶到边界没动算没导航
        assert!(!session.move_highlight(-1));
        assert!(!session.navigated);
        assert!(session.move_highlight(1));
        assert!(session.navigated);
        session.reset(None, candidates(3), 9, 2);
        assert!(!session.navigated);
        // 只有一页时翻页没动，也不算导航
        assert!(!session.turn_page(1));
        assert!(!session.navigated);
        session.reset(None, candidates(12), 9, 2);
        assert!(session.turn_page(1));
        assert!(session.navigated);
    }

    #[test]
    fn sparse_custom_positions_keep_highlight_on_real_candidates() {
        let mut words = candidates(1);
        words[0].kind = CandidateKind::Custom(9);
        let mut session = Session::default();
        session.reset(None, words.clone(), 5, 2);
        assert_eq!((session.page, session.highlighted), (1, 8));
        assert!(!session.turn_page(-1));
        words.extend(candidates(1));
        session.reset(None, words, 5, 2);
        assert_eq!((session.page, session.highlighted), (0, 0));
        assert!(session.turn_page(1));
        assert_eq!((session.page, session.highlighted), (1, 8));
        assert!(session.move_highlight(-1));
        assert_eq!((session.page, session.highlighted), (0, 0));
    }
}
