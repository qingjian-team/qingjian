//! 缓冲区与模式：按键进出、光标移动、表达式 / 英文直输 / 问字等模式判断、标点与上屏链。

use super::*;

impl Engine {
    /// 中文模式下把半角字符转成全角标点；不需要转换返回 `None`。
    pub fn punctuate(&mut self, c: char) -> Option<&'static str> {
        let converted = self.punctuation.convert(c);
        if let Some(text) = converted {
            self.history.record(text);
            self.remember_commit(LastCommit::plain(text));
        } else {
            self.recent_commits.clear();
        }
        self.chain.reset();
        converted
    }

    /// 壳把字符原样透传给应用后告知，用于「数字后的点保持半角」，也记入输入历史。
    pub fn note_passthrough(&mut self, c: char) {
        self.punctuation.note_passthrough(c);
        let text = c.encode_utf8(&mut [0; 4]).to_owned();
        self.history.record(&text);
        self.chain.reset();
        self.remember_commit(LastCommit::plain(&text));
    }

    /// 壳告知光标离开了刚才上屏的位置（切换应用、点了别处、停用输入法）：之后上屏的词按句首记。
    pub fn break_chain(&mut self) {
        self.chain.reset();
        self.recent_commits.clear();
    }

    /// 壳告知：不在组句时按了退格，删的是应用里刚上屏的文字。从最近一次上屏往前数，一次上屏的字删光了就是「可能选错了」的信号：
    /// 接着重打那段拼音选了别的词，那次记的学习就退回去（见 [`Self::apply_retraction`]）。
    /// 删得比记着的几次上屏加起来还多说明在改别处，全忘掉。
    pub fn note_backspace(&mut self) {
        let Some(commit) = self
            .recent_commits
            .iter_mut()
            .rev()
            .find(|c| !c.is_erased())
        else {
            self.recent_commits.clear();
            self.chain.reset();
            return;
        };
        commit.erased += 1;
        if commit.is_erased() {
            // 刚上屏的词没了，它不再是下一个词的上文
            self.chain.reset();
        }
    }

    /// 记一次上屏到最近上屏列表，超出条数丢最早的。
    pub(super) fn remember_commit(&mut self, commit: LastCommit) {
        if commit.chars == 0 {
            return;
        }
        if self.recent_commits.len() >= RECENT_COMMITS {
            self.recent_commits.remove(0);
        }
        self.recent_commits.push(commit);
    }

    pub fn composition(&self) -> &Composition {
        &self.composition
    }

    pub fn push(&mut self, c: char) {
        self.composition.push(c);
    }

    pub fn backspace(&mut self) -> bool {
        self.composition.backspace()
    }

    pub fn clear(&mut self) {
        self.composition.clear();
        self.chain.leave_buffer();
    }

    pub fn delete_forward(&mut self) -> bool {
        self.composition.delete_forward()
    }

    pub fn move_cursor_left(&mut self) -> bool {
        self.composition.move_left()
    }

    pub fn move_cursor_right(&mut self) -> bool {
        self.composition.move_right()
    }

    pub fn move_cursor_home(&mut self) {
        self.composition.move_home();
    }

    pub fn move_cursor_end(&mut self) {
        self.composition.move_end();
    }

    /// 是否处在表达式模式（缓冲区以表达式键、缺省 `v` 开头）。此时壳应把数字和运算符也交给 [`Self::push`]，而不是当选词键。
    pub fn expression_mode(&self) -> bool {
        self.modes().is_expression(self.composition.text())
    }

    /// 英文直输段：缓冲区里有拼音以外的字符（`no-way`），整段原样上屏、不解析拼音。
    /// 表达式模式与问字模式优先于它。
    pub fn raw_mode(&self) -> bool {
        is_raw(self.composition.text(), self.modes(), self.shuangpin)
    }

    /// 是否处在问字模式（缓冲区以问字键、缺省 `u`，或 `?` 开头）：拼音问题由云端答，十六进制码点本地答。
    pub fn question_mode(&self) -> bool {
        self.modes().is_question(self.composition.text())
    }

    /// 问字模式下正在敲的还可能是 Unicode 码点（前缀后为空，或到目前为止全是十六进制 / 开头 `+`）：
    /// 此时壳应把数字交给 [`Self::push`] 而不是当选词键。
    pub fn unicode_entry(&self) -> bool {
        let text = self.composition.text();
        self.modes().is_question(text)
            && shortcut::could_be_unicode(self.modes().question_body(text))
    }

    /// 缓冲区里只有一个 `?`：还没决定是问字还是中文问号。壳在下一个键不是字母时应把它还原成 `？`。
    pub fn bare_question(&self) -> bool {
        self.composition.text() == QUESTION_PREFIX.to_string()
    }

    /// 用一段完整拼音替换当前缓冲区，供 CLI 和测试一次性喂入。
    pub fn set_input(&mut self, input: &str) {
        self.composition.clear();
        for c in input.chars() {
            self.composition.push(c);
        }
    }

    /// 放弃当前拼音，原样返回给壳（通常是用户按回车要上屏字母本身）。
    pub fn take_raw(&mut self) -> String {
        // 纠错生效时用户仍按了回车：这个串就是要原样打的，记下来以后不再纠它
        let scope = self.composition.scope().to_owned();
        if !self.english_mode && self.active_correction(&scope).is_some() {
            self.learner.record_raw(&scope);
            // 缓存里还是「要纠」，清掉让下次重算
            *self.correction_cache.borrow_mut() = None;
        }
        let raw = self.composition.text().to_owned();
        self.log_commit(&raw, &raw, InputSource::Raw);
        // 原样上屏的是个英文词（`gist`）：记进个人英文词表，下次直接出候选。
        // 双拼下全部键都能解成完整音节的（`nihc`）不是英文，是用户要原样打出双拼键
        let english_word = looks_like_english_word(&raw, self.english_mode)
            && (self.english_mode || self.decode(&raw).is_none_or(|d| !d.is_complete()));
        if english_word {
            self.learner.learn_english(&raw);
        }
        self.meter_commit(&raw, InputSource::Raw, english_word);
        self.composition.clear();
        self.remember_commit(LastCommit::plain(&raw));
        self.punctuation.note_committed(&raw);
        self.history.record(&raw);
        self.chain.reset();
        raw
    }
}
