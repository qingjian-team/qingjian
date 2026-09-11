//! 缓冲区与模式：按键进出、光标移动、表达式 / 英文直输 / 问字等模式判断、标点与上屏链。

use super::*;

/// 直通字符攒到这么多就先写一条，免得长时间纯英文输入时一条攒得没边。
const MAX_PENDING_PASSTHROUGH: usize = 200;

impl Engine {
    /// 中文模式下把半角字符转成全角标点；不需要转换返回 `None`。
    pub fn punctuate(&mut self, c: char) -> Option<&'static str> {
        let converted = self.punctuation.convert(c);
        if let Some(text) = converted {
            self.history.record(text);
            self.remember_commit(LastCommit::plain(text));
            // 全角标点也是文本流的一部分，与直通字符攒在一起
            self.passthrough_pending.push_str(text);
        } else {
            self.recent_commits.clear();
        }
        self.chain.reset();
        converted
    }

    /// 壳把字符原样透传给应用后告知，用于「数字后的点保持半角」，也记入输入历史与输入日志（攒成一条 `passthrough`）。
    pub fn note_passthrough(&mut self, c: char) {
        self.punctuation.note_passthrough(c);
        let text = c.encode_utf8(&mut [0; 4]).to_owned();
        self.history.record(&text);
        self.chain.reset();
        self.remember_commit(LastCommit::plain(&text));
        self.passthrough_pending.push(c);
        if self.passthrough_pending.chars().count() >= MAX_PENDING_PASSTHROUGH {
            self.flush_passthrough();
        }
    }

    /// 把攒着的直通字符写成一条输入日志。上屏、上文断开、会话记录前都调，保证日志里的顺序与真实顺序一致。
    pub(super) fn flush_passthrough(&mut self) {
        if self.passthrough_pending.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.passthrough_pending);
        self.logger.record(InputLogEntry::Passthrough { text });
    }

    /// 壳告知光标离开了刚才上屏的位置（切换应用、点了别处、停用输入法）：之后上屏的词按句首记。
    /// 上次断开之后有过上屏才往输入日志记一条 `break`，连着失焦几次只记一次。
    pub fn break_chain(&mut self) {
        self.chain.reset();
        self.recent_commits.clear();
        self.flush_passthrough();
        if self.committed_since_break {
            self.committed_since_break = false;
            self.logger.record(InputLogEntry::Break {
                app: self.application.clone(),
            });
        }
    }

    /// 壳告知正在输入的应用（macOS bundle identifier / Windows exe 名），写进输入日志；不知道就给 `None`。
    pub fn set_application(&mut self, app: Option<String>) {
        self.application = app;
    }

    pub fn application(&self) -> Option<&str> {
        self.application.as_deref()
    }

    /// 壳翻了一页候选：记进这段组句的翻页数（写进输入日志，候选质量的隐式信号）。
    pub fn note_page_turn(&mut self) {
        self.page_turns = self.page_turns.saturating_add(1);
    }

    /// 往输入日志记一条会话信息（版本、平台、本地模型开没开）。壳在启动和打开日志时调。
    pub fn log_session(&mut self, version: &str, platform: &str) {
        self.flush_passthrough();
        self.logger.record(InputLogEntry::Session {
            v: INPUT_LOG_VERSION,
            version: version.to_owned(),
            platform: platform.to_owned(),
            model: self.has_sentence_scorer(),
            scheme: self.scheme_key(),
        });
    }

    /// 双拼方案的键（`xiaohe`），全拼为空；输入日志用。
    pub(super) fn scheme_key(&self) -> String {
        self.shuangpin
            .map_or_else(String::new, |s| s.key().to_owned())
    }

    /// 组句里要删东西了：第一次删之前把缓冲区留个快照，上屏时对比最终键串，不同就是一次重打（`retype`）。
    fn note_edit(&mut self) {
        if self.retype_snapshot.is_none() && !self.composition.is_empty() {
            self.retype_snapshot = Some(self.composition.text().to_owned());
        }
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
        if self.composition.is_empty() {
            // 新一段组句：从这一键起算耗时、翻页与重打
            self.composition_started = Some(Instant::now());
            self.page_turns = 0;
            self.retype_snapshot = None;
        }
        self.composition.push(c);
    }

    pub fn backspace(&mut self) -> bool {
        self.note_edit();
        self.composition.backspace()
    }

    pub fn clear(&mut self) {
        self.composition.clear();
        self.chain.leave_buffer();
        // 壳给的光标前文只对这段组句有效，下一段第一键再读
        self.rescoring_before = None;
        self.retype_snapshot = None;
        self.composition_started = None;
        self.page_turns = 0;
    }

    pub fn delete_forward(&mut self) -> bool {
        self.note_edit();
        self.composition.delete_forward()
    }

    /// 删掉光标前的一个音节（壳里 ⌥⌫）：全拼按最优切分的最后一个音节连同它后面的 `'`，切不动的尾巴整个删；
    /// 双拼两键一音节，落单的一键单删；英文直输段 / 表达式 / 问字里删最后一段字母或数字，标点一次删一个。
    /// 光标在开头时返回 `false`。
    pub fn delete_syllable_backward(&mut self) -> bool {
        self.note_edit();
        let cursor = self.composition.cursor();
        let before = &self.composition.text()[..cursor];
        let plain = self.raw_mode() || self.expression_mode() || self.question_mode();
        let len = unit_len_before(before, self.shuangpin.is_some(), plain);
        self.composition.delete_before_cursor(len)
    }

    /// 光标往左跳一个音节（壳里 ⌥←），边界与 [`Self::delete_syllable_backward`] 相同。已在开头返回 `false`。
    pub fn move_cursor_syllable_left(&mut self) -> bool {
        let cursor = self.composition.cursor();
        let before = &self.composition.text()[..cursor];
        let plain = self.raw_mode() || self.expression_mode() || self.question_mode();
        let len = unit_len_before(before, self.shuangpin.is_some(), plain);
        len > 0 && (0..len).all(|_| self.composition.move_left())
    }

    /// 光标往右跳一个音节（壳里 ⌥→）：跳过紧跟的 `'`，再跳过一个音节。已在末尾返回 `false`。
    pub fn move_cursor_syllable_right(&mut self) -> bool {
        let cursor = self.composition.cursor();
        let after = &self.composition.text()[cursor..];
        let plain = self.raw_mode() || self.expression_mode() || self.question_mode();
        let len = unit_len_after(after, self.shuangpin.is_some(), plain);
        len > 0 && (0..len).all(|_| self.composition.move_right())
    }

    /// 删掉光标前的全部拼音（壳里 ⌘⌫），光标后的留着。光标在开头时返回 `false`。
    pub fn delete_to_start(&mut self) -> bool {
        self.note_edit();
        let cursor = self.composition.cursor();
        self.composition.delete_before_cursor(cursor)
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
        if raw.is_empty() {
            // 壳在回车 / 失焦时不管有没有在组句都会来一趟：空的不记日志、不计统计
            self.clear();
            self.chain.reset();
            return raw;
        }
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

/// 光标后的第一个「单位」占几个字节：先跳过紧跟的 `'`，再算一个音节；规则同 [`unit_len_before`]。
fn unit_len_after(after: &str, shuangpin: bool, plain: bool) -> usize {
    let trimmed = after.trim_start_matches('\'');
    let separators = after.len() - trimmed.len();
    let Some(first) = trimmed.chars().next() else {
        return separators;
    };
    if plain || !first.is_ascii_lowercase() {
        let run = if first.is_ascii_alphanumeric() {
            trimmed
                .chars()
                .take_while(char::is_ascii_alphanumeric)
                .map(char::len_utf8)
                .sum()
        } else {
            first.len_utf8()
        };
        return separators + run;
    }
    if shuangpin {
        let run = trimmed
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || *c == ';')
            .count();
        return separators + run.min(2);
    }
    let syllable = match segment_longest_prefix(trimmed) {
        Ok((segmentations, _)) => segmentations
            .first()
            .and_then(|s| s.syllables.first())
            .map_or(1, |s| s.text.len()),
        Err(_) => 1,
    };
    separators + syllable
}

/// 光标前的最后一个「单位」占几个字节：拼音里是一个音节（连同它后面的 `'`），见 [`Engine::delete_syllable_backward`]。
fn unit_len_before(before: &str, shuangpin: bool, plain: bool) -> usize {
    let trimmed = before.trim_end_matches('\'');
    let separators = before.len() - trimmed.len();
    let Some(last) = trimmed.chars().last() else {
        return separators;
    };
    // 直输段 / 表达式 / 问字，或末尾不是字母：字母数字连成一段删，其他字符一次一个
    if plain || !last.is_ascii_lowercase() {
        let run = if last.is_ascii_alphanumeric() {
            trimmed
                .chars()
                .rev()
                .take_while(char::is_ascii_alphanumeric)
                .map(char::len_utf8)
                .sum()
        } else {
            last.len_utf8()
        };
        return separators + run;
    }
    if shuangpin {
        // 两键一音节：连着的键数是奇数说明末尾落单一键
        let run = trimmed
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_lowercase() || *c == ';')
            .count();
        return separators + if run % 2 == 1 { 1 } else { 2 };
    }
    let syllable = match segment_longest_prefix(trimmed) {
        Ok((_, tail)) if !tail.is_empty() => tail.len(),
        Ok((segmentations, _)) => segmentations
            .first()
            .and_then(|s| s.syllables.last())
            .map_or(1, |s| s.text.len()),
        Err(_) => 1,
    };
    separators + syllable
}
