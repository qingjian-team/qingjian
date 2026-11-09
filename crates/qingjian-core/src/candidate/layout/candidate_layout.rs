//! 云候选分页布局；可显式选择 AI 候选优先，自定义短语保留固定位置。

use super::super::{Candidate, CandidateKind};
use super::Cell;

/// 本地候选 + 云端词的分页排布；旧入口放在末尾，优先入口按模型顺序置顶。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CandidateLayout {
    /// 本地候选，顺序就是 Engine 排好的顺序。
    local: Vec<Candidate>,

    /// 已到的云端词；同文的普通本地候选在显示时隐藏。
    cloud: Vec<Candidate>,

    /// 每页几格。
    page_size: usize,

    /// 配置的云端槽位数；0 表示不要云端词。
    slots: usize,

    /// 当前云候选是否采用优先入口；旧平台调用保持原末尾布局。
    cloud_first: bool,
}

impl CandidateLayout {
    pub fn new(local: Vec<Candidate>, page_size: usize, slots: usize) -> Self {
        Self {
            local,
            cloud: Vec::new(),
            page_size: page_size.max(1),
            slots,
            cloud_first: false,
        }
    }

    pub fn page_size(&self) -> usize {
        self.page_size
    }

    pub fn local(&self) -> &[Candidate] {
        &self.local
    }

    pub fn local_mut(&mut self) -> &mut [Candidate] {
        &mut self.local
    }

    pub fn cloud(&self) -> &[Candidate] {
        &self.cloud
    }

    pub fn cloud_mut(&mut self) -> &mut [Candidate] {
        &mut self.cloud
    }

    /// 当前布局是否使用显式候选优先入口。
    pub fn cloud_first(&self) -> bool {
        self.cloud_first
    }

    /// 第一页最多给云端几格，避开自定义短语的固定位置；问字模式整页都给云端。
    pub fn capacity(&self) -> usize {
        if self.local.is_empty() {
            self.page_size
        } else if !self.cloud_first {
            let fixed = self
                .local
                .iter()
                .filter_map(|c| match c.kind {
                    CandidateKind::Custom(n) => Some(n),
                    _ => None,
                })
                .max()
                .unwrap_or(1);
            self.slots.min(self.page_size.saturating_sub(fixed.max(1)))
        } else {
            let fixed = self
                .local
                .iter()
                .filter(
                    |c| matches!(c.kind, CandidateKind::Custom(n) if n > 0 && n <= self.page_size),
                )
                .count();
            self.slots.min(self.page_size.saturating_sub(fixed))
        }
    }

    /// 保留旧平台入口：过滤本地同文候选，补入第一页末尾。
    pub fn set_cloud(&mut self, words: Vec<Candidate>) -> usize {
        self.replace_cloud(words, false)
    }

    /// 按模型顺序置顶；同文候选合并，自定义短语固定位置优先。返回采纳的条数。
    pub fn set_cloud_first(&mut self, words: Vec<Candidate>) -> usize {
        self.replace_cloud(words, true)
    }

    fn replace_cloud(&mut self, words: Vec<Candidate>, first: bool) -> usize {
        self.cloud_first = first;
        self.cloud.clear();
        let capacity = self.capacity();
        for mut word in words {
            if self.cloud.len() >= capacity
                || self.local.iter().any(|c| {
                    c.text == word.text && (!first || matches!(c.kind, CandidateKind::Custom(_)))
                })
                || self.cloud.iter().any(|c| c.text == word.text)
            {
                continue;
            }
            word.kind = CandidateKind::Cloud;
            self.cloud.push(word);
        }
        self.cloud.len()
    }

    fn visible_local(&self) -> impl Iterator<Item = &Candidate> {
        self.local
            .iter()
            .filter(|c| !self.cloud.iter().any(|word| word.text == c.text))
    }

    /// 固定位置先放好，普通候选按当前入口的顺序填入。
    pub fn cells(&self) -> Vec<Cell<'_>> {
        let local_len = self.len()
            - if self.cloud_first {
                0
            } else {
                self.cloud.len()
            };
        let mut cells = vec![Cell::Empty; local_len];
        for candidate in &self.local {
            if let CandidateKind::Custom(position) = candidate.kind
                && let Some(cell) = position.checked_sub(1).and_then(|i| cells.get_mut(i))
            {
                *cell = Cell::Local(candidate);
            }
        }
        let mut normal = self
            .cloud
            .iter()
            .filter(|_| self.cloud_first)
            .map(Cell::Cloud)
            .chain(
                self.visible_local()
                    .filter(|c| !matches!(c.kind, CandidateKind::Custom(_)))
                    .map(Cell::Local),
            );
        for cell in &mut cells {
            if matches!(cell, Cell::Empty)
                && let Some(candidate) = normal.next()
            {
                *cell = candidate;
            }
        }
        if !self.cloud_first {
            let first = cells.len().min(self.page_size - self.cloud.len());
            cells.splice(first..first, self.cloud.iter().map(Cell::Cloud));
        }
        cells
    }

    pub fn len(&self) -> usize {
        let fixed = self
            .local
            .iter()
            .filter_map(|c| match c.kind {
                CandidateKind::Custom(position) => Some(position),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        if self.cloud_first {
            fixed.max(self.visible_local().count() + self.cloud.len())
        } else {
            fixed.max(self.local.len()) + self.cloud.len()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn pages(&self) -> usize {
        self.len().div_ceil(self.page_size)
    }

    /// 第 `index` 格的候选；越界返回 `None`。
    pub fn candidate(&self, index: usize) -> Option<&Candidate> {
        self.cells().get(index).and_then(|cell| cell.candidate())
    }

    /// 第 `page` 页的格子。
    pub fn page(&self, page: usize) -> Vec<Cell<'_>> {
        self.cells()
            .into_iter()
            .skip(page * self.page_size)
            .take(self.page_size)
            .collect()
    }
}
