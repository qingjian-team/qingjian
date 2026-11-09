//! 分几次选完一段拼音时，前面选好的词先留在组句里不上屏：显示在拼音前面，整段选完或回车时一起交给应用；
//! 退格先撤回最后一次选择，把那段拼音接回来重选，全部撤回后才删字母。

mod selection;

pub(super) use selection::HeldSelection;

use super::Engine;
use crate::composition::Composition;

impl Engine {
    /// 选词之后：缓冲区还有拼音就把这次的文本留下、返回空串；选完了连同前面留着的一起返回。
    /// `before` 是选词前的缓冲区，`None` 表示这次不留（直接交给应用）。
    pub(super) fn hold_or_release(&mut self, before: Option<Composition>, text: String) -> String {
        match before {
            Some(before) if !self.composition.is_empty() => {
                let consumed = before.text().len() - self.composition.text().len();
                self.held.push(HeldSelection {
                    text,
                    keys: before.prefix(consumed),
                });
                String::new()
            }
            _ => self.take_held() + &text,
        }
    }

    /// 取走留着的全部文本（按选的先后连起来），组句里的拼音不动。
    /// 壳在「上屏候选后紧接着插入别的东西」（组句中敲标点）时先把它交给应用，免得标点插到前面。
    pub fn take_held(&mut self) -> String {
        self.held.drain(..).map(|held| held.text).collect()
    }

    /// 前面选好、还没交给应用的文本；组句显示时接在拼音前面。
    pub fn held_text(&self) -> String {
        self.held.iter().map(|held| held.text.as_str()).collect()
    }

    /// 退格撤回最后一次选择：那段拼音接回缓冲区开头，那次上屏记的学习按「整个删掉了」处理，
    /// 接着重选别的词时退回（与应用里退格删掉刚上屏的词一样，见 [`Self::note_backspace`]）。没有可撤的返回 `false`。
    pub(super) fn undo_selection(&mut self) -> bool {
        let Some(held) = self.held.pop() else {
            return false;
        };
        self.composition.prepend(&held.keys);
        if let Some(commit) = self
            .recent_commits
            .iter_mut()
            .rev()
            .find(|commit| !commit.is_erased())
        {
            commit.erased = commit.chars;
        }
        self.chain.reset();
        true
    }

    /// 选词后剩下拼音时先不上屏（缺省开）。回放输入日志时关掉：日志里选词后的退格删的是字母。
    pub fn set_hold_selections(&mut self, on: bool) {
        self.hold_selections = on;
        if !on {
            self.held.clear();
        }
    }
}
