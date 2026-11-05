//! 上屏后的联想（#861）：中文词上屏、组句结束后，候选条接着放可能跟在后面的词，点了直接上屏并接着联想。
//! 联想词由 Core 的 [`qingjian_core::Engine::follow_ups`] 给；只在候选条上显示，空格、回车、标点照常交给应用。

use super::Session;
use crate::frame::{Frame, FrameCandidate};

/// 候选条上一次放几个联想词：一屏放得下，再多的排在后面也很少有人往后翻。
const FOLLOW_UP_COUNT: usize = 8;

impl Session {
    /// 刚上屏了 `committed`、组句也结束了：把联想词接在 `frame` 的候选后面。开关关着、工具框开着、日语模式不给。
    pub(super) fn offer_follow_ups(&mut self, committed: &str, frame: &mut Frame) {
        if !self.follow_ups
            || self.tools_open
            || self.kana.is_some()
            || committed.is_empty()
            || !self.engine.composition().is_empty()
        {
            return;
        }
        let words = self.engine.follow_ups(FOLLOW_UP_COUNT);
        if words.is_empty() {
            return;
        }
        self.follow_up_at = Some(self.completions.len());
        for word in words {
            frame.candidates.push(FrameCandidate::plain(&word));
            self.completions.push((0, word));
        }
        self.after_commit = Some(committed.to_owned());
    }

    /// 点了联想词 `word`：上屏并记学习，接着给它后面的联想。
    pub(super) fn select_follow_up(&mut self, word: &str) -> Frame {
        let commit = self.engine.commit_follow_up(word);
        let mut frame = self.refresh(commit.clone());
        self.offer_follow_ups(&commit, &mut frame);
        frame
    }
}
