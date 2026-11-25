//! 组句中「修饰键 + 数字」的快捷键：上屏译词。与 macOS 壳对齐。

use qingjian_core::{Candidate, CandidateList};
use qingjian_platform::protocol::KeyModifiers;

use super::Effect;
use crate::dispatch::Router;

impl Router {
    /// 配到哪组就上屏第一 / 第二个译词；哪组都不是返回 `None`，按普通键处理。
    pub(super) fn apply_digit_shortcut(
        &mut self,
        digit: usize,
        chord: KeyModifiers,
    ) -> Option<Effect> {
        if chord == KeyModifiers::default() {
            return None;
        }
        let (first, second) = self.config.translation_keys;
        let effect = if chord == first {
            self.commit_translation_on_page(digit, 0)
        } else if chord == second {
            self.commit_translation_on_page(digit, 1)
        } else {
            return None;
        };
        Some(effect)
    }

    /// 上屏第 `digit` 个候选的第 `sense` 条译文；没有那条译文就吞掉按键不动。
    fn commit_translation_on_page(&mut self, digit: usize, sense: usize) -> Effect {
        let Some(candidate) = self.annotated_candidate_on_page(digit) else {
            tracing::debug!(digit, "这一格没有候选");
            return Effect::Navigated;
        };
        match self.engine.commit_translation(&candidate, sense) {
            Some(text) => Effect::Changed(Some(text)),
            None => {
                tracing::debug!(digit, sense, "这个候选没有这条译文");
                Effect::Navigated
            }
        }
    }

    /// 当前页第 `digit` 个候选（1 起）。数字超出本页就不算——不然每页只有 5 个时按 7
    /// 会取到下一页的候选，把用户看不见的那个词的译词上屏。
    fn candidate_on_page(&self, digit: usize) -> Option<Candidate> {
        let page_size = self.config.page_size;
        let page = self.highlight / page_size;
        let index = page * page_size + page_slot(digit, page_size)?;
        self.layout_candidate(index)
    }

    /// 同上，补上译文（布局里存的是查询原样，译文画页时才补）。
    pub(super) fn annotated_candidate_on_page(&self, digit: usize) -> Option<Candidate> {
        let candidate = self.candidate_on_page(digit)?;
        let mut list = CandidateList {
            items: vec![candidate],
        };
        self.engine.annotate(&mut list);
        list.items.pop()
    }
}

/// 数字键在这一页里对应的下标（0 起）；不在本页（0 或者超过每页个数）就是 `None`。
pub(super) fn page_slot(digit: usize, page_size: usize) -> Option<usize> {
    (digit >= 1 && digit <= page_size).then(|| digit - 1)
}

#[cfg(test)]
mod tests {
    use super::page_slot;

    #[test]
    fn a_digit_outside_the_page_has_no_slot() {
        assert_eq!(page_slot(1, 9), Some(0));
        assert_eq!(page_slot(9, 9), Some(8));
        // 每页 9 个时候选窗口只画 1–9，第 10 个是下一页的
        assert_eq!(page_slot(10, 9), None);
        // 每页调小之后，超出的数字不能顺着往下取到下一页的候选
        assert_eq!(page_slot(2, 1), None);
        assert_eq!(page_slot(7, 5), None);
        assert_eq!(page_slot(0, 9), None);
    }
}
