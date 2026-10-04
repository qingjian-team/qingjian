//! 自动空格（`[general] auto_space`）的上下文：光标前一个字符是什么，接下来交给应用的文本前面要不要补空格。

use super::Engine;
use crate::spacing::needs_space;

impl Engine {
    /// 壳把 `next` 交给应用之前调（上屏候选、原样上屏、直通字符都算）。与上文是中西文交界时把补的空格记成一次直通
    /// （退格撤销按它计数）并返回 true，壳在 `next` 前面插一个空格；没开自动空格、不知道上文时返回 false。
    pub fn auto_space_before(&mut self, next: &str) -> bool {
        let Some(first) = next.chars().next() else {
            return false;
        };
        if !self.auto_space
            || !self
                .previous_char()
                .is_some_and(|prev| needs_space(prev, first))
        {
            return false;
        }
        self.note_passthrough(' ');
        true
    }

    /// 最近一条没被退格删掉的上屏的末字；还没上屏过时用组句开始时壳读到的应用前文。
    /// 删了一半的上屏、焦点离开后（记录已清空）又不在组句时都不知道光标前是什么，返回 `None`。
    fn previous_char(&self) -> Option<char> {
        for commit in self.recent_commits.iter().rev() {
            match commit.erased {
                0 => return commit.tail,
                erased if erased >= commit.chars => continue,
                _ => return None,
            }
        }
        if self.composition.is_empty() {
            return None;
        }
        self.rescoring_before.as_deref()?.chars().last()
    }
}
