//! 按键怎么作用到 Engine / 高亮上。分流规则与 macOS 壳的 `handle_text` / `handle_command` 对齐。

use qingjian_core::candidate::arrows::{self, Arrow, ArrowAction, Focus};
use qingjian_core::{CandidateKind, QUESTION_PREFIX, punctuation, shortcut};
use qingjian_platform::HighlightKeys;
use qingjian_platform::protocol::KeyEvent;

use super::{Effect, codes, with_prefix};
use crate::dispatch::Router;

impl Router {
    /// 功能键靠键码，其余靠字符。组句中修饰键 + 数字是快捷键；带 Ctrl / Alt / Win 而没配到快捷键的键归应用。
    /// 表达式模式里 Shift + 数字打的是 `^ * ( )`，不当快捷键。
    pub(crate) fn apply_key(&mut self, event: &KeyEvent) -> Effect {
        if let Some(effect) = self.apply_panel_key(event) {
            return effect;
        }
        let combo = self.config.predict;
        // 句号键是 VK_OEM_PERIOD；ASCII 0x2E 对应 Delete。
        let key = if combo.key == '.' {
            0xBE
        } else {
            combo.key.to_ascii_uppercase() as u32
        };
        if self.composing()
            && self.config.cloud_enabled
            && self.engine.prediction_enabled()
            && event.virtual_key == key
            && event.modifiers.chord() == combo.modifiers.into()
        {
            if self.request_manual_prediction() {
                // 显式要求刷新时回到第一页，让新的 AI 首选可以置顶。
                self.highlight = 0;
                self.navigated = false;
            }
            return Effect::Navigated;
        }
        if self.composing()
            && !self.engine.expression_mode()
            && let Some(digit) = codes::digit_key(event.virtual_key)
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
            match event.virtual_key {
                // 回车交给应用：文本流里是一个段落边界（macOS 壳同样记）
                codes::RETURN => self.engine.note_passthrough('\n'),
                // 删的是应用里刚上屏的字：自动空格据此知道光标前还是不是那个字
                codes::BACK => self.engine.note_backspace(),
                _ => {}
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
            codes::DOWN if self.config.two_area_arrows() => self.two_area_key(Arrow::Down),
            codes::DOWN => {
                // 横排矩阵开着：↓ 展开成矩阵 / 往下换行；否则按 highlight_keys 移高亮
                if self.config.grid_keys() {
                    self.move_rows(1);
                    Effect::Navigated
                } else {
                    self.vertical_key(1)
                }
            }
            // 高亮还在第一个、没动过时 ↑ 没处可去，拿来开符号面板（矩阵开着时也一样，`↓` 照旧展开矩阵）
            // 方向键分两区时候选里的 ↑ 是回拼音行，要在拼音行里再按一次才开面板
            codes::UP
                if (!self.config.two_area_arrows() || self.focus == Focus::Pinyin)
                    && self.open_panel_from_key() =>
            {
                Effect::Navigated
            }
            codes::UP if self.config.two_area_arrows() => self.two_area_key(Arrow::Up),
            codes::UP => {
                if self.config.grid_keys() {
                    // ↑ 顶在第一排再往上：回到第一个候选（不是毫无动作）
                    if !self.move_rows(-1) {
                        self.jump_to_first();
                    }
                    Effect::Navigated
                } else {
                    self.vertical_key(-1)
                }
            }
            codes::NEXT => {
                self.page(1);
                Effect::Navigated
            }
            codes::PRIOR => {
                self.page(-1);
                Effect::Navigated
            }
            codes::LEFT if self.config.two_area_arrows() => self.two_area_key(Arrow::Left),
            codes::RIGHT if self.config.two_area_arrows() => self.two_area_key(Arrow::Right),
            codes::LEFT => self.horizontal_key(-1),
            codes::RIGHT => self.horizontal_key(1),
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

    /// 横排、高亮移动键选 `←` / `→` 时（`RouterConfig::two_area_arrows`）方向键照 Core 的 `arrows::route` 在拼音行与候选两区之间走，
    /// 规则与 macOS 壳的 `two_area_arrow` 同一份。没有候选时 ← / → 照旧移拼音光标。
    fn two_area_key(&mut self, arrow: Arrow) -> Effect {
        if self.candidate_count() == 0 {
            return match arrow {
                Arrow::Left | Arrow::Right => {
                    self.move_caret(if arrow == Arrow::Left { -1 } else { 1 });
                    Effect::Changed(None)
                }
                Arrow::Up | Arrow::Down => Effect::Navigated,
            };
        }
        match arrows::route(arrow, self.focus, self.matrix()) {
            ArrowAction::MoveCursor(delta) => {
                self.move_caret(delta);
                return Effect::Changed(None);
            }
            ArrowAction::MoveHighlight(delta) => self.move_cells_or_highlight(delta),
            ArrowAction::MoveRow(delta) => {
                self.move_rows(delta);
            }
            ArrowAction::Expand => {
                self.move_rows(1);
            }
            ArrowAction::Focus(focus) => self.focus = focus,
            ArrowAction::Stay => {}
        }
        Effect::Navigated
    }

    /// `↑` / `↓`：按 `[shortcut] highlight_keys` 移高亮。高亮键配成 `←` / `→` 时它俩没活干，
    /// 这一下吃掉但不动——不吃掉会漏给应用去挪插入符。
    fn vertical_key(&mut self, step: isize) -> Effect {
        if self.config.highlight_keys == HighlightKeys::UpDown {
            self.move_highlight(step);
        }
        Effect::Navigated
    }

    /// `←` / `→`：缺省只走拼音光标；高亮键配成 `←` / `→` 或横排矩阵开着时两键一起管，
    /// 把「拼音行 + 候选栏」当成一条从左到右的序列走——拼音行在左边，所以 `→` 先走光标、
    /// 到末位溢出成选词，`←` 先退候选、到首位回光标（macOS 壳的 `move_cells` 是同一套规则）。
    fn horizontal_key(&mut self, step: isize) -> Effect {
        if self.config.highlight_keys == HighlightKeys::UpDown && !self.config.grid_keys() {
            self.move_caret(step);
            return Effect::Changed(None);
        }
        if step > 0 {
            if self.move_caret(1) {
                return Effect::Changed(None);
            }
            self.move_cells_or_highlight(1);
            return Effect::Navigated;
        }
        if self.highlight > 0 {
            self.move_cells_or_highlight(-1);
            return Effect::Navigated;
        }
        // 高亮已在首位：把这一下还给拼音光标；光标也在开头时没有去处，照样吃掉
        if self.move_caret(-1) {
            return Effect::Changed(None);
        }
        Effect::Navigated
    }

    /// 拼音光标左右移一格。已在开头 / 末尾时返回 `false`，调用方据此溢出到另一个动作。
    fn move_caret(&mut self, step: isize) -> bool {
        if step < 0 {
            self.engine.move_cursor_left()
        } else {
            self.engine.move_cursor_right()
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
            return Effect::Changed(self.push_letter(c));
        }
        if c.is_ascii_uppercase() {
            let raw = self.composing().then(|| self.engine.take_raw());
            let effect = self.pass_through(c);
            return with_prefix(raw, effect, c);
        }
        if !self.composing() {
            return self.apply_punctuation(c, event);
        }
        self.apply_printable(c, event)
    }

    /// 字母进缓冲区，带上形码的两条规则：满码再敲字母先把高亮候选顶上屏，敲满唯一的全码直接上屏。
    /// 返回这一键上屏的文本（两条都触发时是两段连起来）。
    fn push_letter(&mut self, c: char) -> Option<String> {
        let topped = self.engine.code_tops(c).then(|| self.commit_highlighted());
        self.engine.push(c);
        let committed: String = topped
            .into_iter()
            .chain(self.engine.commit_unique_code())
            .collect();
        (!committed.is_empty()).then_some(committed)
    }

    /// 当前模式开着全角就让 Core 转（数字后的 `.` 与小键盘的键保持半角）；转不了的原样交给应用并告知 Core。
    fn apply_punctuation(&mut self, c: char, event: &KeyEvent) -> Effect {
        let english = event.modifiers.caps || event.modifiers.english_mode;
        // 数字与字母的自动空格要在 punctuate 之前判：它转不了时清掉上屏记录，之后就不知道上文了
        let lead = c.is_ascii_alphanumeric() && self.engine.auto_space_before(&c.to_string());
        if !codes::is_keypad(event.virtual_key)
            && self.full_width_for(english)
            && let Some(text) = self.engine.punctuate(c)
        {
            return Effect::Changed(Some(text.to_owned()));
        }
        self.engine.note_passthrough(c);
        if lead {
            Effect::Changed(Some(format!(" {c}")))
        } else {
            Effect::Passthrough
        }
    }

    /// 字符原样交给应用；开了自动空格且与上文是中西文交界时改由我们连同空格一起插入（macOS 壳的 `pass_through` 同理）。
    fn pass_through(&mut self, c: char) -> Effect {
        let lead = self.engine.auto_space_before(&c.to_string());
        self.engine.note_passthrough(c);
        if lead {
            Effect::Changed(Some(format!(" {c}")))
        } else {
            Effect::Passthrough
        }
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
                let lead = self.engine.auto_space_before(&c.to_string());
                self.engine.note_passthrough(c);
                Effect::Changed(Some(if lead { format!(" {c}") } else { c.to_string() }))
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
        // 翻页键要排在连字符前面，否则 `-` 永远翻不回上一页；翻不动时 `-` 照旧是单词里的连字符（`well-known`）
        if composing
            && let Some(step) = self.page_step(event)
            && (c != '-' || self.can_page(step))
        {
            self.page(step);
            return Effect::Navigated;
        }
        if c.is_ascii_alphabetic()
            || (composing && (c.is_ascii_digit() || matches!(c, '_' | '\'' | '-')))
        {
            self.engine.push(c);
            return Effect::Changed(None);
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
        // 空格整段原样上屏，空格本身也要在（`hello, world`）；高亮挪到邮箱地址上时只上屏地址。
        if self.engine.raw_mode() {
            if c == ' ' {
                let address = self
                    .layout_candidate(self.highlight)
                    .is_some_and(|candidate| candidate.kind == CandidateKind::Shortcut);
                let committed = self.commit_highlighted();
                if address {
                    return Effect::Changed(Some(committed));
                }
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
        // `-` `=` 翻不动时落回原字符（与 macOS 一致）；`[` `]` `,` `.` 翻不动照旧吃键，落字会往文档里插标点
        if let Some(step) = self.page_step(event)
            && (!matches!(c, '-' | '=') || self.can_page(step))
        {
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
        // 这条上屏行为由 `[general] punct_commits` 控制（缺省关：标点进英文直输段），单用形码时总是上屏（五笔的习惯）。
        // 不会转的符号（`-` `/` `@`）仍进英文直输段；`'` 是隔音符，任何模式都进缓冲区。
        if (c != '\''
            && (expression
                || self.engine.question_mode()
                || ((self.config.punct_commits || self.engine.composing_code())
                    && punctuation::converts(c))))
            || self.engine.in_aux()
        {
            // 高亮候选没吃完拼音时它留在组句里：连同前面留着的先交给应用，标点才接得在后面
            let committed = self.commit_highlighted() + &self.engine.take_held();
            let effect = self.apply_punctuation(c, event);
            return with_prefix(Some(committed), effect, c);
        }
        self.engine.push(c);
        Effect::Changed(None)
    }

    /// 数字键在当前页对应的格子下标；这一页没有这一格（`gpt6` 只有三个候选）返回 `None`，数字当内容进缓冲区。
    /// 云端词还没到的占位格算有：按了不算，免得结果一到就选错。
    pub(super) fn slot_index(&self, digit: usize) -> Option<usize> {
        let page_size = self.config.page_size;
        let index = self.highlight / page_size * page_size + digit - 1;
        (digit <= page_size && index < self.candidate_count()).then_some(index)
    }

    /// 上屏高亮候选；没有候选时缓冲原样上屏，形码空码（编码查不到词）时清掉编码、什么都不上屏。
    fn commit_highlighted(&mut self) -> String {
        if self.engine.composing_code() && self.candidate_count() == 0 {
            self.engine.clear();
            return String::new();
        }
        match self.commit_index(self.highlight) {
            Some(text) => text,
            None => self.engine.take_raw(),
        }
    }

    pub(in crate::dispatch) fn composing(&self) -> bool {
        !self.engine.composition().is_empty()
    }

    /// 横排矩阵展开着时按阅读顺序移一格（跨行、跳过空位），否则逐个移动高亮（到页边自动翻页）。
    fn move_cells_or_highlight(&mut self, delta: isize) {
        if self.grid.is_some() {
            self.move_cells(delta);
        } else {
            self.move_highlight(delta);
        }
    }
}
