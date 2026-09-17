//! 按键处理：把 X keysym 翻译成引擎动作。返回 true = 吞掉。

use qingjian_platform::Modifiers;

use super::Host;

/// fcitx5(X11)修饰键位：Shift / Ctrl / Alt(Mod1)/ Super(Mod4)。
const SHIFT: u32 = 1 << 0;
const CTRL: u32 = 1 << 2;
const ALT: u32 = 1 << 3;
const SUPER: u32 = 1 << 6;

/// 配置里的修饰键组合（macOS 词汇）翻成 X 修饰位：option→Alt,command→Super。
fn mods_mask(m: Modifiers) -> u32 {
    (if m.shift { SHIFT } else { 0 })
        | (if m.control { CTRL } else { 0 })
        | (if m.option { ALT } else { 0 })
        | (if m.command { SUPER } else { 0 })
}

/// keysym 是主键区或小键盘的数字 1-9 吗？是则给出 0 起的页内序号。
fn digit_offset(keyval: u32) -> Option<usize> {
    match keyval {
        0x31..=0x39 => Some((keyval - 0x31) as usize),
        0xffb1..=0xffb9 => Some((keyval - 0xffb1) as usize),
        _ => None,
    }
}

/// 数字键 1-9 被 Shift 按着时出的符号 keysym（美式布局，下标即 0 起的页内序号）。
const SHIFTED_DIGITS: [u32; 9] = [
    0x21, // !
    0x40, // @
    0x23, // #
    0x24, // $
    0x25, // %
    0x5e, // ^
    0x26, // &
    0x2a, // *
    0x28, // (
];

/// Shift 按着时数字键在 X 下出的是符号 keysym（美式布局 `!` `@` `#` …）：映射回 0 起的页内序号。
/// 只给修饰键快捷键（译词第二组、删候选缺省带 Shift）用，普通标点路径不受影响。
fn shifted_digit_offset(keyval: u32) -> Option<usize> {
    SHIFTED_DIGITS.iter().position(|&s| s == keyval)
}

/// 同一个物理数字键的「另一个」keysym:keysym 由事件当时的修饰状态决定，Shift+1 按下到达 `!`、
/// 先松 Shift 再松键则松键到达 `1`。松键记账要认这对变体，否则漏无头 keyup 还留陈账。
/// 根治是 shim 传 keycode，见 docs/design/linux-fcitx5.md。
fn shift_counterpart(keyval: u32) -> Option<u32> {
    if let Some(offset) = shifted_digit_offset(keyval) {
        return Some(0x31 + offset as u32);
    }
    if (0x31..=0x39).contains(&keyval) {
        return Some(SHIFTED_DIGITS[(keyval - 0x31) as usize]);
    }
    None
}

impl Host {
    /// 按键处理。返回 true = 吞掉。keyval 是 X keysym。
    pub fn key(&mut self, keyval: u32, state: u32, release: bool) -> bool {
        self.maybe_reload_config();
        const SHIFT_KEYS: std::ops::RangeInclusive<u32> = 0xffe1..=0xffe2;
        // Shift 轻点切中英：按下预备，中间没夹别的键、松开时兑现（macOS 同款手感）。
        if SHIFT_KEYS.contains(&keyval) {
            if release {
                if self.shift_armed {
                    self.shift_armed = false;
                    let on = !self.engine.english_mode();
                    self.engine.set_english_mode(on);
                    self.refresh();
                }
            } else {
                self.shift_armed = true;
            }
            return false; // 修饰键本身永远透传，应用要看 Shift 状态
        }
        // 记账认「本 keysym 或它的 Shift 变体」：同一物理键按下与松开的 keysym 可能不同。
        let twin = shift_counterpart(keyval);
        if release {
            // 松键与按下对称：按下被吞的键松键也吞，按下透传的松键照样透传。
            // 只看「按下时吞过没有」，不看当前组句状态——上屏类的键（空格/回车/数字/Esc）
            // 按下就结束了组句，按组句状态判会把它们的松键漏给应用（无头 keyup）。
            if let Some(i) = self
                .swallowed_presses
                .iter()
                .position(|&k| k == keyval || Some(k) == twin)
            {
                self.swallowed_presses.swap_remove(i);
                return true;
            }
            return false;
        }
        self.shift_armed = false;
        let swallow = self.press(keyval, state);
        if swallow {
            if !self.swallowed_presses.contains(&keyval) {
                self.swallowed_presses.push(keyval);
            }
        } else {
            // 这次按下透传了：同物理键的陈账（切焦点后没等到松键的）一并清掉，免得吞错以后的松键。
            self.swallowed_presses
                .retain(|&k| k != keyval && Some(k) != twin);
        }
        swallow
    }

    /// 按下事件的路由（松键在 [`Self::key`] 里按「按下吞过没有」对称处理，不进这里）。
    fn press(&mut self, keyval: u32, state: u32) -> bool {
        const CTRL_ALT_SUPER: u32 = CTRL | ALT | SUPER;
        // 修饰键+数字快捷键（只在组句中认）：译词上屏（缺省 Alt=第一个，Alt+Shift=第二个）、删候选（缺省 Shift）。
        // 表达式模式(v2^3)里 ⇧+数字打的是 ^ * ( )，不当快捷键；配置成翻页键的字符也让位(如 "!(")。
        if self.composing()
            && !self.engine.expression_mode()
            && keyval != u32::from(self.page_keys.0)
            && keyval != u32::from(self.page_keys.1)
            && let Some(offset) = digit_offset(keyval).or_else(|| shifted_digit_offset(keyval))
        {
            let pressed = state & (SHIFT | CTRL | ALT | SUPER);
            if pressed != 0 {
                let (first, second) = self.translation_mods;
                if pressed == mods_mask(first) {
                    self.commit_translation_on_page(offset, 0);
                    return true;
                }
                if pressed == mods_mask(second) {
                    self.commit_translation_on_page(offset, 1);
                    return true;
                }
                if pressed == mods_mask(self.delete_mods) {
                    self.forget_on_page(offset);
                    return true;
                }
            }
        }
        // 修饰键+方向/退格（只在组句中认，macOS 同款，见 docs/user/getting-started/keys.md）：
        // Alt=按音节移光标/删音节（⌥ 对应），Super=光标到头尾/删到头（⌘ 对应）。
        if self.composing() {
            let pressed = state & (SHIFT | CTRL | ALT | SUPER);
            match (keyval, pressed) {
                (0xff51 | 0xff96, ALT) => {
                    self.engine.move_cursor_syllable_left();
                    self.refresh();
                    return true;
                }
                (0xff53 | 0xff98, ALT) => {
                    self.engine.move_cursor_syllable_right();
                    self.refresh();
                    return true;
                }
                (0xff51 | 0xff96, SUPER) => {
                    self.engine.move_cursor_home();
                    self.refresh();
                    return true;
                }
                (0xff53 | 0xff98, SUPER) => {
                    self.engine.move_cursor_end();
                    self.refresh();
                    return true;
                }
                (0xff08, ALT) => {
                    self.engine.delete_syllable_backward();
                    self.refresh();
                    return true;
                }
                (0xff08, SUPER) => {
                    self.engine.delete_to_start();
                    self.refresh();
                    return true;
                }
                _ => {}
            }
        }
        if state & CTRL_ALT_SUPER != 0 {
            return false;
        }
        let composing = self.composing();
        // 英文直输段（缓冲区里已有 `-` 这类字符）：可见字符一律追加，空格/回车整段原样上屏。
        let raw = composing && self.engine.raw_mode();
        // 表达式模式（v 开头）：数字与运算符进缓冲区，不当选词/翻页键。
        let expression = composing && self.engine.expression_mode();
        // 问字模式（u 开头）敲的还可能是码点（u4e00、u+1f600）：数字与 + 进缓冲区而不是选词。
        let question = composing && self.engine.question_mode();
        let unicode = question && self.engine.unicode_entry();
        match keyval {
            // a-z：进缓冲区。英文候选关着（全局或本应用）时的英文模式是纯直通，字母不进缓冲区。
            0x61..=0x7a => {
                if self.engine.english_mode() && !self.english_candidates_here() && !composing {
                    self.engine.note_passthrough(keyval as u8 as char);
                    return false;
                }
                self.engine.push(keyval as u8 as char);
                self.refresh();
                true
            }
            // 直输段里的可见字符（数字、标点，翻页键字符也算）：字面追加。
            _ if raw && (0x21..=0x7e).contains(&keyval) => {
                self.engine.push(keyval as u8 as char);
                self.refresh();
                true
            }
            // 表达式模式的数字与运算符（+ - * / ^ 括号小数点）：进缓冲区。
            _ if expression
                && (0x21..=0x7e).contains(&keyval)
                && qingjian_core::shortcut::is_expression_char(keyval as u8 as char) =>
            {
                self.engine.push(keyval as u8 as char);
                self.refresh();
                true
            }
            // 问字模式的码点输入：数字（含 0）与 + 进缓冲区（十六进制字母走上面的 a-z）。
            _ if unicode && (matches!(keyval, 0x30..=0x39 | 0x2b)) => {
                self.engine.push(keyval as u8 as char);
                self.refresh();
                true
            }
            _ if unicode && (0xffb0..=0xffb9).contains(&keyval) => {
                self.engine.push((b'0' + (keyval - 0xffb0) as u8) as char);
                self.refresh();
                true
            }
            // 英文组句里的数字、大写、下划线：进缓冲区（win32、McDonald、snake_case），不选词不打断。
            _ if composing
                && self.engine.english_mode()
                && matches!(keyval, 0x30..=0x39 | 0x41..=0x5a | 0x5f) =>
            {
                self.engine.push(keyval as u8 as char);
                self.refresh();
                true
            }
            // 音节分隔符 '：组句中进缓冲区（xi'an、英文 don't），不当标点。
            0x27 if composing => {
                self.engine.push('\'');
                self.refresh();
                true
            }
            // 微软/搜狗双拼的 `;` 是 ing 键：末尾有落单声母时进缓冲区，其他时候还是标点。
            0x3b if composing && self.engine.takes_semicolon() => {
                self.engine.push(';');
                self.refresh();
                true
            }
            // 数字 1-9（含小键盘）：组句中按页内序号上屏。
            _ if composing && !expression && !unicode && digit_offset(keyval).is_some() => {
                let offset = digit_offset(keyval).expect("刚匹配过");
                let index = self.page * self.layout.page_size() + offset;
                if self.layout.candidate(index).is_some() {
                    self.commit_index(index);
                }
                true
            }
            // 空格：中文模式总是上屏高亮候选；英文模式只在动过高亮后才选，
            // 没动过就把敲的字母原样上屏（词表里没有的词不被补全替换）。
            // 直输段与英文模式里空格本身还要交给应用（`hello, world` 里的空格要在），
            // 中文模式空格是选词键、照吞——macOS/Windows 同款口径。
            0x20 if composing => {
                if raw {
                    self.commit_index(self.highlighted);
                    self.engine.note_passthrough(' ');
                    return false;
                }
                if self.engine.english_mode() {
                    if self.navigated {
                        self.commit_index(self.highlighted);
                    } else {
                        self.commit_raw();
                    }
                    self.engine.note_passthrough(' ');
                    return false;
                }
                self.commit_index(self.highlighted);
                true
            }
            // 回车：拼音原文上屏。
            0xff0d | 0xff8d if composing => {
                self.commit_raw();
                true
            }
            // 退格：组句中删一个字符。
            0xff08 if composing => {
                self.engine.backspace();
                self.refresh();
                true
            }
            // 不组句时的退格删的是已上屏的词：记「选错了」信号供纠错学习，按键仍透传给应用真删。
            0xff08 => {
                self.engine.note_backspace();
                false
            }
            // Delete / 小键盘 Delete：光标后向前删。
            0xffff | 0xff9f if composing => {
                self.engine.delete_forward();
                self.refresh();
                true
            }
            // Esc：放弃本次输入。
            0xff1b if composing => {
                self.engine.clear();
                self.refresh();
                true
            }
            // 翻页：配置的键对（缺省 `[` `]`）与 PageUp / PageDown（含小键盘）、方向键上下。
            // 先于 `-` 的直输段入口：用户显式配 "-=" 时 `-` 是翻页键。
            // 英文模式不认字符翻页键：标点一律半角透传（选词靠方向键，翻页还有 PageUp/Down）。
            _ if composing
                && !self.engine.english_mode()
                && keyval == u32::from(self.page_keys.0) =>
            {
                self.turn_page(-1);
                true
            }
            _ if composing
                && !self.engine.english_mode()
                && keyval == u32::from(self.page_keys.1) =>
            {
                self.turn_page(1);
                true
            }
            // 组句中敲 `-`：进入英文直输段(`no-way`)，不当翻页键——翻页键见配置 `[general] page_keys`。
            // 表达式的 `-` 是运算符，已在上面进缓冲；问字模式不进直输段。
            0x2d if composing && !question => {
                self.engine.push('-');
                self.refresh();
                true
            }
            0xff55 | 0xff9a if composing => {
                self.turn_page(-1);
                true
            }
            0xff56 | 0xff9b if composing => {
                self.turn_page(1);
                true
            }
            // Tab：中文模式翻下一页，英文模式选中高亮的词(macOS 口径；
            // Linux 无整句补全，macOS「有补全先接受」那臂在这儿用不上)。
            0xff09 if composing => {
                if self.engine.english_mode() {
                    self.commit_index(self.highlighted);
                } else {
                    self.turn_page(1);
                }
                true
            }
            // ⇧+Tab 到达时是 ISO_Left_Tab(0xfe20)，在 0xff00 兜底段之外，
            // 不显式接管就会漏给应用、反向跳焦点作废组句。macOS 口径：上一页。
            0xfe20 if composing => {
                self.turn_page(-1);
                true
            }
            // 方向键上下（含小键盘）：移动高亮（跨页时页码跟随）。macOS/用户文档同款口径。
            0xff52 | 0xff97 if composing => {
                self.move_highlight(-1);
                true
            }
            0xff54 | 0xff99 if composing => {
                self.move_highlight(1);
                true
            }
            // 方向键左右（含小键盘）：移动拼音光标——候选只按光标之前的拼音计算（ni|hao 出 你）。
            0xff51 | 0xff96 if composing => {
                self.engine.move_cursor_left();
                self.refresh();
                true
            }
            0xff53 | 0xff98 if composing => {
                self.engine.move_cursor_right();
                self.refresh();
                true
            }
            // Home / End（含小键盘）：光标到开头 / 末尾。
            0xff50 | 0xff95 if composing => {
                self.engine.move_cursor_home();
                self.refresh();
                true
            }
            0xff57 | 0xff9c if composing => {
                self.engine.move_cursor_end();
                self.refresh();
                true
            }
            // Shift 按住打的大写字母：临时打英文——拼音原样上屏，字母本身透传给应用。
            0x41..=0x5a => {
                if composing {
                    self.commit_raw();
                }
                self.engine.note_passthrough(keyval as u8 as char);
                false
            }
            // 其余可打印键 = 标点/符号：组句中先上屏高亮候选，然后转全角；
            // 转不了的（半角规则如数字后的点）原样透传。空闲时同一条路。
            0x21..=0x7e => {
                let c = keyval as u8 as char;
                // 英文模式标点一律半角（macOS 口径）：不走全角转换，组句先把敲的字母原样上屏，
                // 标点本身交给应用。
                if self.engine.english_mode() {
                    if composing {
                        self.commit_raw();
                    }
                    self.engine.note_passthrough(c);
                    return false;
                }
                // 中文组句敲半角标点（翻页键已在前截）：进缓冲区，整段成为英文直输段
                // (`hello,` `dui'ma?`)——与 macOS 实现、docs/user/input/shortcuts.md 口径一致。
                if composing && !question && !expression {
                    self.engine.push(c);
                    self.refresh();
                    return true;
                }
                // 问字/表达式里的非模式字符：先把高亮候选上屏，再按标点处理。
                if composing {
                    self.commit_index(self.highlighted);
                }
                match self.engine.punctuate(c) {
                    Some(full_width) => {
                        self.push_commit(full_width.to_owned());
                        true
                    }
                    None => {
                        // 透传：字符本身不吞。若组句中已上屏候选，顺序由 shim 保证——
                        // 它对未吞掉的键也先取走 pending_commit 发出去，再放行按键。
                        self.engine.note_passthrough(c);
                        false
                    }
                }
            }
            // 修饰键本身（Ctrl / Alt / Super / CapsLock、AltGr、Num_Lock…）按下永远透传，
            // 应用要看修饰状态。ISO_* 级别键在 0xfe00 段，由下面的兜底透传。
            0xffe1..=0xffee | 0xff7f => false,
            // 组句期间功能键区（0xff00 段：Tab / Home / F 键…）剩下的一律接管吞掉，
            // 否则应用会动光标、丢焦点，组句跟着作废（macOS 同款口径）。
            // 段外的键（XF86 媒体键、AltGr 打出的非 ASCII 字符）不吞：那不是编辑动作。
            _ if composing && (0xff00..=0xffff).contains(&keyval) => true,
            _ => false,
        }
    }

    /// 私密输入（密码框）：学习与输入日志静音，排序不变。
    pub fn set_private(&mut self, private: bool) {
        self.engine.set_private(private);
    }

    /// 上屏当前页第 `offset` 格（鼠标点选走这里）。
    pub fn select_on_page(&mut self, offset: usize) {
        let index = self.page * self.layout.page_size() + offset;
        if self.layout.candidate(index).is_some() {
            self.commit_index(index);
        }
    }

    /// 会话重置（切窗/切输入法）：缓冲区未上屏内容直接丢弃，学习落盘。
    pub fn reset(&mut self) {
        self.engine.clear();
        self.clear_view();
        self.pending_commit = None;
        // HOST 是进程级单例（不分 InputContext）：按住 Shift 点击切窗后松开，会在新窗口
        // 静默切中英——切窗时必须撤销待兑现的 Shift 轻点。
        self.shift_armed = false;
        self.navigated = false;
        self.flush();
    }
}
