//! 按键怎么作用到 Engine / 高亮上。中英文模式的分流规则与 macOS 壳的 `handle_text` / `handle_command` 对齐：
//! Caps Lock 亮着是英文模式，字母大小写只看 Shift。

use qingjian_platform::protocol::KeyEvent;

use super::{Effect, Router, keys};

impl Router {
    /// 功能键靠键码，其余靠字符。英文模式组词中 Caps Lock 灭了（或候选关了）：敲的字母先原样上屏，别把它们当拼音。
    /// 终端、编辑器这类应用（`[apps] english_candidates_off`，按宿主 exe 名）里英文模式是纯直通。
    /// 组句中修饰键 + 数字是快捷键（译词上屏 / 删候选）；带 Ctrl / Alt / Win 而没配到快捷键的键归应用。
    pub(super) fn apply_key(&mut self, event: &KeyEvent) -> Effect {
        if self.composing()
            && let Some(digit) = keys::digit_key(event.virtual_key)
            && let Some(effect) = self.apply_digit_shortcut(digit, event.modifiers.chord())
        {
            return effect;
        }
        if event.modifiers.has_command_key() {
            return Effect::Passthrough;
        }
        let Some(c) = event.character.filter(|c| !c.is_control()) else {
            return self.apply_function_key(event);
        };
        // Caps Lock 亮着无论中英模式都直接出大写英文（无候选）；候选只在持久英文模式、Caps 灭、应用允许时给。
        let caps = event.modifiers.caps;
        let english = caps || event.modifiers.english_mode;
        let english_candidates = event.modifiers.english_mode
            && !caps
            && self.config.english_candidates_in(self.focused_app());
        let flushed = (self.composing() && !english_candidates && self.engine.english_mode())
            .then(|| self.engine.take_raw());
        self.engine.set_english_mode(english_candidates);
        let effect = if english {
            self.apply_english(c, english_candidates)
        } else {
            self.apply_chinese(c, event)
        };
        with_prefix(flushed, effect, c)
    }

    /// 退格 / Esc / 回车 / Tab / 方向键；没在组句时都交还应用。
    fn apply_function_key(&mut self, event: &KeyEvent) -> Effect {
        if !self.composing() {
            return Effect::Passthrough;
        }
        match event.virtual_key {
            keys::BACK => {
                self.engine.backspace();
                Effect::Changed(None)
            }
            keys::ESCAPE => {
                self.engine.clear();
                Effect::Changed(None)
            }
            keys::RETURN => Effect::Changed(Some(self.engine.take_raw())),
            // 英文模式 Tab 选中高亮的词；中文模式有整句补全时接受它，否则交还应用（缩进 / 跳焦点）。
            keys::TAB if self.engine.english_mode() => {
                Effect::Changed(Some(self.commit_highlighted()))
            }
            keys::TAB => match self.sentence.take() {
                Some(sentence) => Effect::Changed(Some(self.engine.accept_prediction(&sentence))),
                None => Effect::Passthrough,
            },
            keys::DOWN => {
                self.move_highlight(1);
                Effect::Navigated
            }
            keys::UP => {
                self.move_highlight(-1);
                Effect::Navigated
            }
            keys::NEXT => {
                self.page(1);
                Effect::Navigated
            }
            keys::PRIOR => {
                self.page(-1);
                Effect::Navigated
            }
            keys::LEFT => {
                self.engine.move_cursor_left();
                Effect::Changed(None)
            }
            keys::RIGHT => {
                self.engine.move_cursor_right();
                Effect::Changed(None)
            }
            keys::HOME => {
                self.engine.move_cursor_home();
                Effect::Changed(None)
            }
            keys::END => {
                self.engine.move_cursor_end();
                Effect::Changed(None)
            }
            _ => Effect::Passthrough,
        }
    }

    /// 中文模式：小写字母进拼音；按住 Shift 的大写字母是临时打英文，组句中先把拼音原样上屏再交给应用；
    /// 其余字符只在组句中才处理。
    fn apply_chinese(&mut self, c: char, event: &KeyEvent) -> Effect {
        if c.is_ascii_uppercase() {
            let raw = self.composing().then(|| self.engine.take_raw());
            self.engine.note_passthrough(c);
            return with_prefix(raw, Effect::Passthrough, c);
        }
        if c.is_ascii_lowercase() {
            self.engine.push(c);
            return Effect::Changed(None);
        }
        if !self.composing() {
            return Effect::Passthrough;
        }
        self.apply_printable(c, event)
    }

    /// 英文模式（Caps Lock 亮）。开着候选：字母（以及组词中的数字、`_` `'` `-`）进缓冲区，候选来自英文词表；
    /// 空格 / 标点先把敲的字母原样上屏（方向键动过高亮之后空格才选高亮的词，打 kubectl 这类词表没有的词不会被替换），
    /// 再把这个键给应用。关着候选：纯直通，字母由我们插入（大小写按 Shift），其他键交给应用。
    fn apply_english(&mut self, c: char, candidates: bool) -> Effect {
        let composing = self.composing();
        if !candidates {
            let raw = composing.then(|| self.engine.take_raw());
            self.engine.note_passthrough(c);
            let effect = if c.is_ascii_alphabetic() {
                Effect::Changed(Some(c.to_string()))
            } else {
                Effect::Passthrough
            };
            return with_prefix(raw, effect, c);
        }
        if c.is_ascii_alphabetic()
            || (composing && (c.is_ascii_digit() || matches!(c, '_' | '\'' | '-')))
        {
            self.engine.push(c);
            return Effect::Changed(None);
        }
        let committed = composing.then(|| {
            if c == ' ' && self.navigated {
                self.commit_highlighted()
            } else {
                self.engine.take_raw()
            }
        });
        self.engine.note_passthrough(c);
        with_prefix(committed, Effect::Passthrough, c)
    }

    /// 组句中的可打印键：数字选当前页第 N 个（没有候选时进缓冲区），翻页键对翻页，空格上屏高亮，
    /// 其余（半角标点等）进英文直输段。
    fn apply_printable(&mut self, c: char, event: &KeyEvent) -> Effect {
        if let Some(digit) = keys::digit(event)
            && self.candidate_count() > 0
        {
            let page_size = self.config.page_size;
            let page = self.highlight / page_size;
            return Effect::Changed(self.commit_index(page * page_size + digit - 1));
        }
        if let Some(step) = keys::page_key(event, self.config.page_keys) {
            self.page(step);
            return Effect::Navigated;
        }
        if c == ' ' {
            return Effect::Changed(Some(self.commit_highlighted()));
        }
        self.engine.push(c);
        Effect::Changed(None)
    }

    /// 上屏高亮的候选；没有候选时拼音（英文模式下是敲的字母）原样上屏。
    fn commit_highlighted(&mut self) -> String {
        match self.commit_index(self.highlight) {
            Some(text) => text,
            None => self.engine.take_raw(),
        }
    }

    fn composing(&self) -> bool {
        !self.engine.composition().is_empty()
    }
}

/// 把先行上屏的文本接到本次按键的结果前面。macOS 上「先上屏、再把这个键交给应用」是两步；Windows 上放行是同步的、
/// 上屏走异步编辑会话，两步一起会让应用先插空格后插词，所以本该放行的键改成由我们连同上屏文本一起插入。
fn with_prefix(prefix: Option<String>, effect: Effect, c: char) -> Effect {
    let Some(mut prefix) = prefix else {
        return effect;
    };
    match effect {
        Effect::Changed(commit) => {
            prefix.push_str(commit.as_deref().unwrap_or_default());
        }
        Effect::Navigated => {}
        Effect::Passthrough => prefix.push(c),
    }
    Effect::Changed(Some(prefix))
}
