//! 组句中「修饰键 + 数字」的快捷键：上屏译词、删候选。与 macOS 壳的 `handle_translation_key` / `handle_delete_key` 对齐。

use qingjian_core::{Candidate, CandidateList};
use qingjian_platform::protocol::KeyModifiers;

use super::{Effect, Router};

impl Router {
    /// 修饰键 + 当前页第 `digit` 个候选：配的哪组就上屏第一 / 第二个译词或删候选；哪组都不是返回 `None`，按普通键处理。
    /// 那格没有候选、或候选没有那条译文时吞掉按键不动。
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
        } else if chord == self.config.delete_keys {
            self.forget_on_page(digit)
        } else {
            return None;
        };
        Some(effect)
    }

    /// 上屏第 `digit` 个候选的第 `sense` 条译文（学习与拼音消耗和选了这个候选一样）。
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

    /// 删掉第 `digit` 个候选：用户词整个删掉，词库词清掉对它的学习记录；然后重新查一遍（排序会变）。
    fn forget_on_page(&mut self, digit: usize) -> Effect {
        let Some(candidate) = self.candidate_on_page(digit) else {
            tracing::debug!(digit, "这一格没有候选，没什么可删");
            return Effect::Navigated;
        };
        let forgotten = self.engine.forget(&candidate);
        let text = &candidate.text;
        let message = if forgotten.user_word {
            format!("已删除用户词「{text}」")
        } else if forgotten.learning {
            format!("已忘掉对「{text}」的学习记录")
        } else {
            format!("「{text}」是词库里的词，也没有学习记录，没什么可删")
        };
        // TODO(windows)：macOS 把这句话显示在拼音行右侧，Windows 候选窗还没有这个位置，先只记日志。
        tracing::info!(%message);
        Effect::Changed(None)
    }

    /// 当前页第 `digit` 个候选（1 起）。
    fn candidate_on_page(&self, digit: usize) -> Option<Candidate> {
        let page_size = self.config.page_size;
        let page = self.highlight / page_size;
        let index = page * page_size + digit.checked_sub(1)?;
        self.layout_candidate(index)
    }

    /// 同上，但带上译文：候选布局里存的是查询原样，译文是画每一页时才补的。
    fn annotated_candidate_on_page(&self, digit: usize) -> Option<Candidate> {
        let candidate = self.candidate_on_page(digit)?;
        let mut list = CandidateList {
            items: vec![candidate],
        };
        self.engine.annotate(&mut list);
        list.items.pop()
    }
}
