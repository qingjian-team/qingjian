//! 打字：按键、选词、删词、回车、空格与补全。

use super::Session;
use crate::frame::{Frame, FrameCandidate};

impl Session {
    /// 不在组句时按光标前的文字给补全（「用户名@」之后的邮箱后缀、刚打完的算式的结果），放进帧的候选；组句中不动。
    pub fn complete(&mut self, before_cursor: &str) -> Frame {
        if !self.engine.composition().is_empty()
            || self.kana.as_ref().is_some_and(|kana| !kana.is_empty())
        {
            return self.refresh(String::new());
        }
        let mut frame = self.refresh(String::new());
        let mut labels = Vec::new();
        for completion in qingjian_core::email_completions(before_cursor) {
            labels.push(completion.domain.to_owned());
            self.completions.push((0, completion.insert));
        }
        for completion in qingjian_core::calculation_completions(before_cursor) {
            labels.push(completion.label);
            self.completions
                .push((completion.replace, completion.insert));
        }
        frame.candidates = labels
            .iter()
            .map(|label| FrameCandidate::plain(label))
            .collect();
        frame
    }

    pub fn push(&mut self, c: char) -> Frame {
        if let Some(kana) = &mut self.kana {
            kana.push(c);
            return self.refresh(String::new());
        }
        self.engine.push(c);
        self.refresh(String::new())
    }

    pub fn backspace(&mut self) -> Frame {
        if let Some(kana) = &mut self.kana {
            kana.backspace();
            return self.refresh(String::new());
        }
        self.engine.backspace();
        self.refresh(String::new())
    }

    /// 选中上一帧的第 `index` 个候选；拼音没吃完就接着组句。
    pub fn select(&mut self, index: usize) -> Frame {
        if !self.completions.is_empty() {
            let (replace, commit) = self.completions.get(index).cloned().unwrap_or_default();
            let mut frame = self.refresh(commit);
            frame.replace = replace;
            return frame;
        }
        if let Some(kana) = &mut self.kana {
            let commit = kana.candidates().get(index).cloned().unwrap_or_default();
            if !commit.is_empty() {
                kana.clear();
            }
            return self.refresh(commit);
        }
        if let Some(commit) = self.select_sentence(index) {
            return self.refresh(commit);
        }
        let commit = match self.candidates.items.get(index) {
            Some(candidate) => self.engine.commit(candidate),
            None => String::new(),
        };
        self.refresh(commit)
    }

    /// 上屏第 `index` 个候选的第 `sense` 条译文；学习与拼音消耗同选了这个候选。没有那条译文就不动。
    pub fn select_translation(&mut self, index: usize, sense: usize) -> Frame {
        let commit = self
            .candidates
            .items
            .get(index)
            .and_then(|candidate| self.engine.commit_translation(candidate, sense))
            .unwrap_or_default();
        self.refresh(commit)
    }

    /// 删掉第 `index` 个候选：用户词整删，词库词清学习记录；结果写进帧的 `notice`。
    pub fn forget(&mut self, index: usize) -> Frame {
        let Some(candidate) = self.candidates.items.get(index).cloned() else {
            return self.refresh(String::new());
        };
        let forgotten = self.engine.forget(&candidate);
        let text = &candidate.text;
        let notice = if forgotten.user_word {
            format!("已删除用户词「{text}」")
        } else if forgotten.learning {
            format!("已忘掉对「{text}」的学习记录")
        } else {
            format!("「{text}」是词库里的词，没什么可删")
        };
        let mut frame = self.refresh(String::new());
        frame.notice = notice;
        frame
    }

    /// 回车：敲的字母原样上屏；日语上屏转好的平假名（与日文输入法回车确定一致）。
    pub fn commit_raw(&mut self) -> Frame {
        if let Some(kana) = &mut self.kana {
            let hiragana = kana.hiragana();
            kana.clear();
            return self.refresh(hiragana);
        }
        let raw = self.engine.take_raw();
        self.refresh(raw)
    }

    /// 清空组句（九宫格的「重输」）。
    pub fn clear(&mut self) -> Frame {
        if let Some(kana) = &mut self.kana {
            kana.clear();
        }
        self.engine.clear();
        self.refresh(String::new())
    }

    /// 组句中按空格：注音的音节还缺声调时空格是一声、进缓冲区（与桌面端一致），否则选第一个候选；
    /// 切不出候选就原样上屏。第一个候选没吃完拼音时选的词留在组句里（见 [`qingjian_core::Engine::take_held`]），接着选后面的。
    pub fn space(&mut self) -> Frame {
        if self.engine.zhuyin_needs_tone() {
            return self.push(' ');
        }
        let selectable = match &self.kana {
            Some(kana) => !kana.candidates().is_empty(),
            None => !self.candidates.items.is_empty(),
        };
        if selectable {
            self.select(0)
        } else {
            self.commit_raw()
        }
    }

    /// 九宫格拼音列里点了一个写法。
    pub fn pin(&mut self, alternative: &str) -> Frame {
        self.engine.pin_nine_key(alternative);
        self.refresh(String::new())
    }
}
