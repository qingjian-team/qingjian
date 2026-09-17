//! 输入会话状态：查询刷新、候选排布、上屏、翻页与高亮、学习落盘。

use qingjian_core::CandidateLayout;

use super::{CLOUD_SLOTS, FLUSH_INTERVAL, Host};

impl Host {
    pub fn composing(&self) -> bool {
        !self.engine.composition().is_empty()
    }

    /// 重新查询并刷新候选排布（每次缓冲区变化后调）。
    pub fn refresh(&mut self) {
        self.attach_loaded_model();
        if !self.composing() {
            self.clear_view();
            return;
        }
        match self.engine.query() {
            Ok(mut query) => {
                self.engine.annotate(&mut query.candidates);
                self.preedit = query.marked_text();
                self.preedit_cursor = query.marked_cursor();
                self.layout =
                    CandidateLayout::new(query.candidates.items, self.page_size, CLOUD_SLOTS);
                self.highlighted = (0..self.layout.len())
                    .find(|&i| self.layout.candidate(i).is_some())
                    .unwrap_or(0);
                self.page = self.highlighted / self.layout.page_size();
                self.navigated = false; // 新一轮查询，高亮未被用户动过
                self.note_displayed_page();
                self.schedule_rescoring(); // 整句路径缺神经分：停键后送后台重排
            }
            Err(error) => {
                // 解析不动（如纯辅音）：preedit 原样显示缓冲区，不出候选。
                tracing::debug!(%error, "查询失败");
                self.preedit = self.engine.composition().text().to_owned();
                self.preedit_cursor = self.preedit.len();
                self.layout = CandidateLayout::new(Vec::new(), self.page_size, CLOUD_SLOTS);
                self.highlighted = 0;
                self.page = 0;
                self.note_displayed_page();
            }
        }
    }

    /// 把当前页的候选告诉 Engine：上屏那一刻它们在屏上，其译词算「见过一轮」（词汇记录）。
    /// 每次重画换掉上一页；窗口空了传空。与 macOS 壳 `render` 里的 `note_displayed` 对齐。
    fn note_displayed_page(&mut self) {
        let start = self.page * self.layout.page_size();
        let end = (start + self.layout.page_size()).min(self.layout.len());
        let page: Vec<_> = (start..end)
            .filter_map(|i| self.layout.candidate(i))
            .collect();
        self.engine.note_displayed(page);
    }

    pub(super) fn clear_view(&mut self) {
        self.layout = CandidateLayout::new(Vec::new(), self.page_size, CLOUD_SLOTS);
        self.highlighted = 0;
        self.page = 0;
        self.preedit.clear();
        self.preedit_cursor = 0;
        self.engine.note_displayed(std::iter::empty());
    }

    /// 上屏第 `index` 格（排布内绝对下标）。
    pub(super) fn commit_index(&mut self, index: usize) {
        let Some(candidate) = self.layout.candidate(index).cloned() else {
            return self.commit_raw();
        };
        let text = self.engine.commit(&candidate);
        self.push_commit(text);
        // 连续组句：上屏后缓冲区还有剩余拼音就接着查，否则收窗。
        self.refresh();
    }

    pub(super) fn commit_raw(&mut self) {
        let raw = self.engine.take_raw();
        self.push_commit(raw);
        self.refresh();
    }

    pub(super) fn push_commit(&mut self, text: String) {
        if text.is_empty() {
            return;
        }
        match &mut self.pending_commit {
            Some(pending) => pending.push_str(&text),
            None => self.pending_commit = Some(text),
        }
        // 学习数据周期落盘：焦点切换即时落之外的兜底，防长会话崩溃丢学习。
        if self.last_flush.elapsed() >= FLUSH_INTERVAL {
            self.engine.flush_learning();
            self.last_flush = std::time::Instant::now();
        }
    }

    pub(super) fn page_count(&self) -> usize {
        self.layout.len().div_ceil(self.layout.page_size()).max(1)
    }

    /// 页码指示 "当前/总页"（1 起）。只有一页时返回空串（不显示）。
    pub fn page_indicator(&self) -> String {
        let total = self.page_count();
        if total <= 1 {
            String::new()
        } else {
            format!("{}/{}", self.page + 1, total)
        }
    }

    pub(super) fn turn_page(&mut self, delta: isize) {
        let pages = self.page_count() as isize;
        let next = (self.page as isize + delta).clamp(0, pages - 1);
        if next != self.page as isize {
            self.page = next as usize;
            self.highlighted = self.page * self.layout.page_size();
            self.navigated = true;
            self.engine.note_page_turn();
            self.note_displayed_page(); // 翻到新页，新页候选算「见过」
        }
    }

    pub(super) fn move_highlight(&mut self, delta: isize) {
        let len = self.layout.len() as isize;
        if len == 0 {
            return;
        }
        let next = (self.highlighted as isize + delta).rem_euclid(len) as usize;
        let old_page = self.page;
        self.highlighted = next;
        self.page = next / self.layout.page_size();
        self.navigated = true;
        if self.page != old_page {
            self.note_displayed_page(); // 高亮换页（如首尾环绕）时同步
        }
    }

    /// shim 告知当前应用（fcitx5 的 program 名）；变了才调。
    pub fn set_program(&mut self, program: &str) {
        program.clone_into(&mut self.program);
    }

    /// 这个应用里英文模式给不给候选：全局开关开着，且应用不在 `[apps] english_candidates_off` 里。
    pub(super) fn english_candidates_here(&self) -> bool {
        self.english_candidates && !self.apps.english_candidates_off(&self.program)
    }

    /// 拼音行显示位置（shim 按它画）：0 = 行内+窗口，1 = 只行内，2 = 只窗口。
    pub fn preedit_display(&self) -> u32 {
        match self.preedit_mode {
            qingjian_platform::PreeditMode::Both => 0,
            qingjian_platform::PreeditMode::Inline => 1,
            qingjian_platform::PreeditMode::Window => 2,
        }
    }

    /// 落盘学习数据（焦点离开时调，与 macOS 的 flush 时机对齐）。
    pub fn flush(&mut self) {
        let _ = &self.data_dir; // user.tsv 路径在 learner 里；flush 由 Engine 统一发。
        self.engine.flush_learning();
    }
    /// 上屏当前页第 `offset` 格的第 `sense` 个译词（译词快捷键）。
    /// 译词缺位就什么都不做，键仍被吞掉（组句中漏给应用会打乱光标，macOS 同款口径）。
    pub(super) fn commit_translation_on_page(&mut self, offset: usize, sense: usize) {
        let index = self.page * self.layout.page_size() + offset;
        let Some(candidate) = self.layout.candidate(index).cloned() else {
            tracing::debug!(offset, "这一格没有候选,没有译词可上屏");
            return;
        };
        match self.engine.commit_translation(&candidate, sense) {
            Some(text) => {
                self.push_commit(text);
                self.refresh();
            }
            None => tracing::debug!(offset, sense, "这个候选没有这条译文"),
        }
    }

    /// 删掉当前页第 `offset` 格候选的用户词/学习记录（删候选快捷键）。
    /// 那格没有候选就什么都不做；删完重新查一遍（排序会变）。
    pub(super) fn forget_on_page(&mut self, offset: usize) {
        let index = self.page * self.layout.page_size() + offset;
        let Some(candidate) = self.layout.candidate(index).cloned() else {
            tracing::debug!(offset, "这一格没有候选,没什么可删");
            return;
        };
        let forgotten = self.engine.forget(&candidate);
        tracing::info!(
            text = %candidate.text,
            user_word = forgotten.user_word,
            learning = forgotten.learning,
            "删候选"
        );
        self.refresh();
    }
}
