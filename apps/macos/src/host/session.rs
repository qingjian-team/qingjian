//! 当前输入会话的 UI 状态：候选排布、高亮、页码、preedit。
//!
//! 放在 Host 里而不是控制器的 ivars 里，是因为联想结果由定时器送达，那时手上没有控制器；
//! 反正 Engine 的缓冲区也是全进程一份，会话状态跟着它走。
//! 候选的分页与云端词的位置由 Core 的 [`CandidateLayout`] 定，这里只管高亮与页码。

use qingjian_core::{Candidate, CandidateLayout, Cell};

use crate::candidates::Preedit;

#[derive(Debug, Default)]
pub struct Session {
    /// 上一次查询的候选排布（本地候选 + 云端词）。
    pub layout: CandidateLayout,

    /// 高亮的格子下标（在整个排布里的绝对位置）。
    pub highlighted: usize,

    /// 当前页。
    pub page: usize,

    /// 这轮查询里用户用方向键 / 翻页键动过高亮。英文模式里空格只在动过之后才选高亮的词，没动过就原样上屏。
    pub navigated: bool,

    /// 候选窗口顶部显示的拼音行（分段 + 光标）。
    pub preedit: Option<Preedit>,
}

impl Session {
    /// 新一轮查询：候选换掉，回到第一页第一项。
    pub fn reset(
        &mut self,
        preedit: Option<Preedit>,
        candidates: Vec<Candidate>,
        page_size: usize,
        slots: usize,
    ) {
        self.preedit = preedit;
        self.layout = CandidateLayout::new(candidates, page_size, slots);
        self.highlighted = 0;
        self.page = 0;
        self.navigated = false;
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
        let next = (current + delta).clamp(0, len as isize - 1) as usize;
        if next == self.highlighted {
            return false;
        }
        self.highlighted = next;
        self.page = next / self.layout.page_size();
        self.navigated = true;
        true
    }

    /// 翻页，高亮落到新页第一项。已在首页 / 末页时返回 false。
    pub fn turn_page(&mut self, delta: isize) -> bool {
        let pages = self.layout.pages().max(1);
        let current = self.page as isize;
        let next = (current + delta).clamp(0, pages as isize - 1) as usize;
        if next == self.page {
            return false;
        }
        self.page = next;
        self.highlighted = next * self.layout.page_size();
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
            })
            .collect()
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
}
