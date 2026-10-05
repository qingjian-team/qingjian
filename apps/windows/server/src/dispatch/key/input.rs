//! 按键怎么作用到 Engine / 高亮上。分流规则与 macOS 壳的 `handle_text` / `handle_command` 对齐。

use qingjian_core::{QUESTION_PREFIX, punctuation, shortcut};
use qingjian_platform::protocol::KeyEvent;

use super::{Effect, codes, with_prefix};
use crate::dispatch::Router;

impl Router {
    /// 功能键靠键码，其余靠字符。组句中修饰键 + 数字是快捷键；带 Ctrl / Alt / Win 而没配到快捷键的键归应用。
    /// 表达式模式里 Shift + 数字打的是 `^ * ( )`，不当快捷键。
    pub(crate) fn apply_key(&mut self, event: &KeyEvent) -> Effect {
        if self.composing()
            && !self.engine.expression_mode()
            && let Some(digit) = codes::digit_key(event.virtual_key)
            && let Some(effect) = self.apply_digit_shortcut(digit, event.modifiers.chord())
        {
            return effect;
        }
        // 横排矩阵开着时 ← / → 归了候选，拼音光标改用 Alt + ← / →（对齐 macOS 的 ⌥← / ⌥→）。
        // 方向键没有字符，得赶在「带 Ctrl / Alt / Win 的键归应用」之前拦下。
        if self.composing()
            && self.config.grid_keys()
            && event.modifiers.alt
            && !event.modifiers.ctrl
            && !event.modifiers.win
        {
            match event.virtual_key {
                codes::LEFT => {
                    self.engine.move_cursor_syllable_left();
                    return Effect::Changed(None);
                }
                codes::RIGHT => {
                    self.engine.move_cursor_syllable_right();
                    return Effect::Changed(None);
                }
                _ => {}
            }
        }
        if event.modifiers.has_command_key() {
            return Effect::Passthrough;
        }
        let Some(c) = event.character.filter(|c| !c.is_control()) else {
            return self.apply_function_key(event);
        };
        // Caps 亮着无论中英模式都直接出大写英文；英文候选只在持久英文模式、Caps 灭、应用允许时给。
        let caps = event.modifiers.caps;
        let english = caps || event.modifiers.english_mode;
        let english_candidates = event.modifiers.english_mode
            && !caps
            && self.config.english_candidates_in(self.focused_app());
        // 缓冲区为空时敲 `?` 先进问字模式（配置 `[shortcut] question_mark`，缺省关），中英文模式都行：
        // 后面跟字母就是在问字，跟别的键就还原成问号。
        if !self.composing() && c == QUESTION_PREFIX && self.engine.takes_question_mark() {
            self.engine.set_english_mode(false);
            self.engine.push(c);
            return Effect::Changed(None);
        }
        // 双拼下 Shift+V / Shift+U 进表达式 / 问字模式（全拼下的 v / u 被音节占了）。
        if !self.composing() && !english && self.engine.takes_mode_letter(c) {
            self.engine.set_english_mode(false);
            self.engine.push(c);
            return Effect::Changed(None);
        }
        let question = self.composing() && self.engine.question_mode();
        // 英文模式下问字：Caps 让字母以大写送来，按小写收进问题。
        let c = if question && english && c.is_ascii_uppercase() {
            c.to_ascii_lowercase()
        } else {
            c
        };
        // 只有一个 `?` 时敲了字母以外的键：还原成问号上屏；空格只是「把这个 ? 上屏」，其他键按没在组句重新分派。
        if question && !c.is_ascii_lowercase() && self.engine.bare_question() {
            let mark = self.restore_bare_question(english);
            if c == ' ' {
                return Effect::Changed(Some(mark));
            }
            return with_prefix(Some(mark), self.apply_key(event), c);
        }
        // 英文组词中候选被关掉（Caps 亮 / 切应用）：敲过的字母先原样上屏。
        let flushed = (self.composing() && !english_candidates && self.engine.english_mode())
            .then(|| self.engine.take_raw());
        self.engine
            .set_english_mode(english_candidates && !question);
        let effect = if english && !question {
            self.apply_english(c, english_candidates, event)
        } else {
            self.apply_chinese(c, event)
        };
        with_prefix(flushed, effect, c)
    }

    /// 缓冲区里只有一个 `?`：清掉，还原成问号（按当前模式的全角设置转）。
    fn restore_bare_question(&mut self, english: bool) -> String {
        self.engine.clear();
        if self.full_width_for(english)
            && let Some(mark) = self.engine.punctuate(QUESTION_PREFIX)
        {
            return mark.to_owned();
        }
        self.engine.note_passthrough(QUESTION_PREFIX);
        QUESTION_PREFIX.to_string()
    }

    /// 退格 / Esc / 回车 / Tab / 方向键；没在组句时都交还应用。
    fn apply_function_key(&mut self, event: &KeyEvent) -> Effect {
        if !self.composing() {
            // 回车交给应用：文本流里是一个段落边界（macOS 壳同样记）
            if event.virtual_key == codes::RETURN {
                self.engine.note_passthrough('\n');
            }
            return Effect::Passthrough;
        }
        // 只有一个 `?` 时按了回车：回车就是「把这个 ? 上屏」，吞掉，否则聊天框会连消息一起发出去；
        // 退格 / Esc 照常删掉它。其他功能键 macOS 壳还原后交给应用，Windows 放行同步、上屏异步，
        // 先动光标再插问号会插错位置，所以还原后一并吞掉。
        if self.engine.bare_question() && !matches!(event.virtual_key, codes::BACK | codes::ESCAPE)
        {
            let english = event.modifiers.caps || event.modifiers.english_mode;
            return Effect::Changed(Some(self.restore_bare_question(english)));
        }
        match event.virtual_key {
            codes::BACK => {
                self.engine.backspace();
                Effect::Changed(None)
            }
            codes::ESCAPE => {
                // 横排矩阵展开着时第一下 Esc 只收回单行（高亮留在原处）
                if self.collapse_grid() {
                    return Effect::Navigated;
                }
                // 辅码态里 Esc 只清码段、拼音留着（与 Core 的 clear_aux 语义一致）
                if self.engine.in_aux() {
                    self.engine.clear_aux();
                } else {
                    self.engine.clear();
                }
                Effect::Changed(None)
            }
            codes::RETURN => {
                if self.engine.is_zhuyin_mode() {
                    if event.modifiers.shift {
                        Effect::Changed(Some(self.engine.take_raw()))
                    } else {
                        Effect::Changed(Some(self.commit_highlighted()))
                    }
                } else {
                    Effect::Changed(Some(self.engine.take_raw()))
                }
            }
            codes::TAB if event.modifiers.shift => {
                self.page(-1);
                Effect::Navigated
            }
            codes::TAB if self.engine.english_mode() => {
                Effect::Changed(Some(self.commit_highlighted()))
            }
            // 中文模式 Tab：有整句补全就接受，否则下一页。
            codes::TAB => match self.sentence.take() {
                Some(sentence) => Effect::Changed(Some(self.engine.accept_prediction(&sentence))),
                None => {
                    self.page(1);
                    Effect::Navigated
                }
            },
            codes::DOWN => {
                // 横排矩阵开着：↓ 展开成矩阵 / 往下换行；否则高亮逐个移动
                if self.config.grid_keys() {
                    self.move_rows(1);
                } else {
                    self.move_highlight(1);
                }
                Effect::Navigated
            }
            codes::UP => {
                if self.config.grid_keys() {
                    // ↑ 顶在第一排再往上：回到第一个候选（不是毫无动作）
                    if !self.move_rows(-1) {
                        self.jump_to_first();
                    }
                } else {
                    self.move_highlight(-1);
                }
                Effect::Navigated
            }
            codes::NEXT => {
                self.page(1);
                Effect::Navigated
            }
            codes::PRIOR => {
                self.page(-1);
                Effect::Navigated
            }
            codes::LEFT => {
                // 横排矩阵开着且归了候选：单行逐个移高亮 / 矩阵里按阅读顺序移；否则照旧移拼音光标
                if self.move_cells_or_highlight(-1) {
                    Effect::Navigated
                } else {
                    self.engine.move_cursor_left();
                    Effect::Changed(None)
                }
            }
            codes::RIGHT => {
                if self.move_cells_or_highlight(1) {
                    Effect::Navigated
                } else {
                    self.engine.move_cursor_right();
                    Effect::Changed(None)
                }
            }
            codes::HOME => {
                self.engine.move_cursor_home();
                Effect::Changed(None)
            }
            codes::END => {
                self.engine.move_cursor_end();
                Effect::Changed(None)
            }
            _ => Effect::Passthrough,
        }
    }

    /// 中文模式：字母进拼音。缺省 Shift 大写是临时打英文——组句中先把拼音原样上屏、字母交给应用；
    /// 配 `[general] shift_letter = "compose"` 时大写也收进缓冲区（Core 按小写匹配、原样上屏时还原大小写）。
    /// 没在组句时的其他字符走全角标点；组句中会转全角的标点先上屏高亮候选再补标点，其余符号仍进英文直输段（见 apply_printable）。
    fn apply_chinese(&mut self, c: char, event: &KeyEvent) -> Effect {
        // 注音模式下数字与 `- ; , . /` 就是键盘上的音节键，跟着进缓冲区。
        let is_zhuyin_key = self.engine.is_zhuyin_mode()
            && (c.is_ascii_digit() || matches!(c, '-' | ';' | ',' | '.' | '/'));
        if c.is_ascii_lowercase()
            || is_zhuyin_key
            || (c.is_ascii_uppercase() && self.config.shift_letter_compose)
        {
            // 辅码态：字母进码段（逐键即筛），不进拼音缓冲区
            if c.is_ascii_lowercase() && self.engine.push_aux_code(c) {
                return Effect::Changed(None);
            }
            // 大写（shift_letter_compose）落在辅码态：先清码段回拼音态（与 Esc 同语义）再收进缓冲区；
            // 码段不清会挂在已变化的缓冲区上继续筛
            if c.is_ascii_uppercase() && self.engine.in_aux() {
                self.engine.clear_aux();
            }
            self.engine.push(c);
            return Effect::Changed(None);
        }
        if c.is_ascii_uppercase() {
            let raw = self.composing().then(|| self.engine.take_raw());
            self.engine.note_passthrough(c);
            return with_prefix(raw, Effect::Passthrough, c);
        }
        if !self.composing() {
            return self.apply_punctuation(c, event);
        }
        self.apply_printable(c, event)
    }

    /// 当前模式开着全角就让 Core 转（数字后的 `.` 与小键盘的键保持半角）；转不了的原样交给应用并告知 Core。
    fn apply_punctuation(&mut self, c: char, event: &KeyEvent) -> Effect {
        let english = event.modifiers.caps || event.modifiers.english_mode;
        if !codes::is_keypad(event.virtual_key)
            && self.full_width_for(english)
            && let Some(text) = self.engine.punctuate(c)
        {
            return Effect::Changed(Some(text.to_owned()));
        }
        self.engine.note_passthrough(c);
        Effect::Passthrough
    }

    /// 英文模式。开着候选：字母进缓冲区，选词与中文模式一样（空格选高亮、数字选当前页第 N 个、翻页键翻页），
    /// 词上屏后空格照样交给应用；数字对应的格子没有候选（词表没有的词、候选不足 N 个）时是标识符的一部分（`foo1`）。
    /// 回车 / 标点先把字母原样上屏。关着候选：字母由我们插入（大小写按 Shift）。
    /// 其他键按英文模式那份全角设置转，转不了的交给应用。
    fn apply_english(&mut self, c: char, candidates: bool, event: &KeyEvent) -> Effect {
        let composing = self.composing();
        if !candidates {
            let raw = composing.then(|| self.engine.take_raw());
            let effect = if c.is_ascii_alphabetic() {
                self.engine.note_passthrough(c);
                Effect::Changed(Some(c.to_string()))
            } else {
                self.apply_punctuation(c, event)
            };
            return with_prefix(raw, effect, c);
        }
        if composing
            && let Some(digit) = codes::digit(event)
            && let Some(index) = self.slot_index(digit)
        {
            return Effect::Changed(self.commit_index(index));
        }
        if c.is_ascii_alphabetic()
            || (composing && (c.is_ascii_digit() || matches!(c, '_' | '\'' | '-')))
        {
            self.engine.push(c);
            return Effect::Changed(None);
        }
        if composing && let Some(step) = codes::page_key(event, self.config.page_keys) {
            self.page(step);
            return Effect::Navigated;
        }
        let committed = composing.then(|| {
            if c == ' ' {
                self.commit_highlighted()
            } else {
                self.engine.take_raw()
            }
        });
        let effect = self.apply_punctuation(c, event);
        with_prefix(committed, effect, c)
    }

    /// 组句中的可打印键：数字选当前页第 N 个（没有这一格就进直输段），翻页键翻页，空格上屏高亮，
    /// 会转全角的标点上屏高亮候选再补标点，其余符号进英文直输段；已在直输段里就一律追加。
    /// 表达式模式（`v1+2`）里数字和运算符进算式；问字模式敲的还可能是码点（`u4e00`、`u+1f600`），数字与 `+` 进缓冲区；
    /// 微软 / 搜狗双拼的 `;` 是 ing 键，末尾有落单声母时进缓冲区。
    fn apply_printable(&mut self, c: char, event: &KeyEvent) -> Effect {
        let expression = self.engine.expression_mode();
        if (expression && shortcut::is_expression_char(c))
            || (self.engine.unicode_entry() && (c.is_ascii_digit() || c == '+'))
            || (c == ';' && self.engine.takes_semicolon())
        {
            self.engine.push(c);
            return Effect::Changed(None);
        }
        // 英文直输段（缓冲区里已有 `-` 这类字符）：可见字符一律追加，数字与翻页键也不再选词 / 翻页；
        // 空格整段原样上屏，空格本身也要在（`hello, world`）。
        if self.engine.raw_mode() {
            if c == ' ' {
                let committed = self.commit_highlighted();
                self.engine.note_passthrough(c);
                return with_prefix(Some(committed), Effect::Passthrough, c);
            }
            if c.is_ascii_graphic() {
                self.engine.push(c);
                return Effect::Changed(None);
            }
        }
        // 辅码触发键：拼音打完整了、这个键也没被键盘方案吃掉 → 进辅码态（触发键不进缓冲区）
        if self.engine.aux_trigger(c) {
            self.engine.enter_aux();
            return Effect::Changed(None);
        }
        // 已经在辅码态里再敲触发键是幂等的，既不上屏候选也不当标点
        if self.engine.in_aux() && c == self.config.aux_code_key {
            return Effect::Changed(None);
        }
        // 码段筛空（例如码敲错一个字母）：空格 / 标点 / 数字都不上屏原始拼音，吞掉停在辅码态等退格
        if self.engine.in_aux() && !self.engine.aux_code().is_empty() && self.candidate_count() == 0
        {
            return Effect::Changed(None);
        }
        if let Some(digit) = codes::digit(event)
            && (!self.engine.is_zhuyin_mode() || self.navigated)
        {
            if let Some(index) = self.slot_index(digit) {
                return Effect::Changed(self.commit_index(index));
            }
            // 问字模式里数字不是问题的一部分：没有这一格就不算
            if self.engine.question_mode() {
                return Effect::Changed(None);
            }
        }
        if let Some(step) = codes::page_key(event, self.config.page_keys) {
            self.page(step);
            return Effect::Navigated;
        }
        if c == ' ' {
            if self.engine.zhuyin_needs_tone() {
                self.engine.push(c);
                return Effect::Changed(None);
            }
            return Effect::Changed(Some(self.commit_highlighted()));
        }
        // 表达式 / 问字模式下的其他字符不进缓冲区（与 macOS 壳一致），辅码态里敲标点同理（码段随之清空）；
        // 普通拼音组句里的标点也一样——会转全角的（`,` `.` `?` `!` 引号、括号这些）先把高亮候选上屏再补标点
        //（`nihao,` 一气打完「你好，」），翻页键已在前面上方拦走，数字后的 `.` 是否保持半角由 Core 判断；
        // 这条上屏行为由 `[general] punct_commits` 控制（缺省开），关掉恢复老行为：标点进英文直输段。
        // 不会转的符号（`-` `/` `@`）仍进英文直输段；`'` 是隔音符，任何模式都进缓冲区。
        if (c != '\''
            && (expression
                || self.engine.question_mode()
                || (self.config.punct_commits && punctuation::converts(c))))
            || self.engine.in_aux()
        {
            let committed = self.commit_highlighted();
            let effect = self.apply_punctuation(c, event);
            return with_prefix(Some(committed), effect, c);
        }
        self.engine.push(c);
        Effect::Changed(None)
    }

    /// 数字键在当前页对应的格子下标；这一页没有这一格（`gpt6` 只有三个候选）返回 `None`，数字当内容进缓冲区。
    /// 云端词还没到的占位格算有：按了不算，免得结果一到就选错。
    fn slot_index(&self, digit: usize) -> Option<usize> {
        let page_size = self.config.page_size;
        let index = self.highlight / page_size * page_size + digit - 1;
        (digit <= page_size && index < self.candidate_count()).then_some(index)
    }

    /// 上屏高亮候选；没有候选时缓冲原样上屏。
    fn commit_highlighted(&mut self) -> String {
        match self.commit_index(self.highlight) {
            Some(text) => text,
            None => self.engine.take_raw(),
        }
    }

    fn composing(&self) -> bool {
        !self.engine.composition().is_empty()
    }

    /// 左右键横排矩阵开着且有候选时归候选（与 macOS 壳的 `move_cells` 一致）：单行里逐个移动高亮
    /// （到页边自动翻页，不展开），展开后按阅读顺序跨行移动——上下键被矩阵占了，单行里只剩左右键能挪高亮。
    /// 照微信输入法的逻辑，`←` 顶在第一个候选上退一级：展开着收回单行（[`Router::move_cells`]），
    /// 单行里交还拼音光标；光标移到最左后 `→` 照旧把光标移回来，直到移回末尾才重新归候选。
    /// 返回是否归了候选（不是的话调用方照旧移动拼音光标）。
    fn move_cells_or_highlight(&mut self, delta: isize) -> bool {
        if !self.config.grid_keys() || self.candidate_count() == 0 {
            return false;
        }
        if self.grid.is_some() {
            self.move_cells(delta);
            return true;
        }
        // 拼音光标已经离开末尾（正在改拼音）：← / → 继续归光标
        if !self.cursor_at_end() {
            return false;
        }
        if delta < 0 && self.highlight == 0 {
            // 顶在第一个候选上：交还拼音光标（去改已输入的拼音）
            return false;
        }
        self.move_highlight(delta);
        true
    }

    /// 拼音光标是否还在缓冲区末尾（没在改拼音）。
    fn cursor_at_end(&self) -> bool {
        let composition = self.engine.composition();
        composition.cursor() >= composition.text().len()
    }
}
