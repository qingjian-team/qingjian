//! IMK 输入控制器：每个输入会话（每个应用的文本框）一个实例。
//!
//! 只做两件事：把按键翻译成 Engine 的调用，把 Engine 返回的候选交给候选窗口。
//! **这里不允许出现排序、词库或翻译逻辑。** 会话状态（候选、高亮、页码）在 [`crate::host::Session`]。

use objc2::rc::{Allocated, Retained};
use objc2::runtime::{AnyObject, Sel};
use objc2::{define_class, msg_send, sel};
use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType, NSMenu};
use objc2_foundation::NSObjectProtocol;
use objc2_input_method_kit::{IMKInputController, IMKServer};
use qingjian_core::{Candidate, QUESTION_PREFIX};
use qingjian_platform::Modifiers;

use super::{TextClient, catch_panic, modifiers, recover_from_panic, secure_input};
use crate::candidates::Preedit;
use crate::host;
use crate::menubar;

define_class!(
    // SAFETY:
    // - IMKInputController 允许子类化，Apple 文档的标准用法就是继承它。
    // - 没有实现 Drop。
    #[unsafe(super(IMKInputController))]
    // 名字要和 Info.plist 的 InputMethodServerControllerClass 一致
    #[name = "QingjianInputController"]
    #[ivars = ()]
    pub struct QingjianInputController;

    impl QingjianInputController {
        /// IMKServer 为每个新会话调用的指定初始化方法，在这里放好 ivars。
        #[unsafe(method_id(initWithServer:delegate:client:))]
        fn init_with_server(
            this: Allocated<Self>,
            server: Option<&IMKServer>,
            delegate: Option<&AnyObject>,
            client: Option<&AnyObject>,
        ) -> Option<Retained<Self>> {
            tracing::info!("新建输入会话");
            let this = this.set_ivars(());
            unsafe { msg_send![super(this), initWithServer: server, delegate: delegate, client: client] }
        }

        /// 所有按键事件都到这里（IMK 第一层协议）。IMK 按控制器实现了哪一层决定路线，实现了这个方法就不会再分发成
        /// `inputText:client:` / `didCommandBySelector:client:`（父类缺省实现也不分发），所以自己分：
        /// Option+数字上屏候选的译文（Option 会把数字键变成 ¡™£ 这类字符，只能按键码认）；命令键按键码映射成原来的选择器；
        /// 其余按事件带的字符走文本路径。返回 true 表示已处理，系统不再把按键交给应用。
        // define_class! 会把返回类型转成 ObjC BOOL，方法体里不能用 `return`，逻辑放在下面的 inherent impl
        #[unsafe(method(handleEvent:client:))]
        fn handle_event(&self, event: Option<&NSEvent>, client: Option<&AnyObject>) -> bool {
            match (event, client) {
                (Some(event), Some(client)) => {
                    let client = TextClient::new(client);
                    // panic 拦下后把缓冲区原样上屏，这个按键交还给应用
                    catch_panic("handleEvent", || self.dispatch_event(event, client))
                        .unwrap_or_else(|| {
                            recover_from_panic(Some(client));
                            false
                        })
                }
                _ => false,
            }
        }

        /// 应用要求立刻结束本次输入（切换焦点、切换输入法等）。
        #[unsafe(method(commitComposition:))]
        fn commit_composition(&self, client: Option<&AnyObject>) {
            let client = client.map(TextClient::new);
            let done = catch_panic("commitComposition", || {
                if let Some(client) = client {
                    self.commit_raw(client);
                }
                host::with(|h| {
                    h.cancel_prediction();
                    h.window.hide();
                });
            });
            if done.is_none() {
                recover_from_panic(client);
            }
        }

        #[unsafe(method(activateServer:))]
        fn activate_server(&self, sender: Option<&AnyObject>) {
            tracing::info!("activateServer");
            let done = catch_panic("activateServer", || {
                // 用户要往 [apps] 里加应用时，从这条日志抄 bundle identifier
                if let Some(bundle) = sender.and_then(|s| TextClient::new(s).bundle_identifier()) {
                    tracing::debug!(%bundle, "当前应用");
                }
                host::with(|h| {
                    h.reload_config_if_changed();
                    h.indicator.activate();
                    h.watch.start();
                });
            });
            if done.is_none() {
                recover_from_panic(None);
            }
        }

        /// 系统输入源菜单（菜单栏旗帜图标）每次展开前来取输入法自己的条目。
        #[unsafe(method_id(menu))]
        fn menu(&self) -> Option<Retained<NSMenu>> {
            host::with(|h| h.menu.ns_menu())
        }

        /// 输入源菜单里点了条目：IMK 转发到控制器，sender 是带 IMKCommandMenuItem 的字典。
        #[unsafe(method(menuAction:))]
        fn menu_action(&self, sender: Option<&AnyObject>) {
            if let Some(action) = menubar::action_from_sender(sender) {
                host::with(|h| h.perform(action));
            }
        }

        #[unsafe(method(deactivateServer:))]
        fn deactivate_server(&self, sender: Option<&AnyObject>) {
            tracing::info!("deactivateServer");
            let client = sender.map(TextClient::new);
            let done = catch_panic("deactivateServer", || {
                if let Some(client) = client {
                    self.commit_raw(client);
                }
                // 切换输入源时无论如何都收掉候选框，不能留一个孤儿窗口在屏幕上
                host::with(|h| {
                    h.cancel_prediction();
                    h.window.hide();
                    h.indicator.deactivate();
                    h.watch.stop();
                    h.engine.break_chain();
                    h.engine.flush_learning();
                    h.last_flush = std::time::Instant::now();
                });
            });
            if done.is_none() {
                recover_from_panic(client);
                // 善后里没做的收尾：学习数据还是要落盘
                host::with(|h| {
                    h.indicator.deactivate();
                    h.watch.stop();
                    h.engine.flush_learning();
                });
            }
        }
    }

    unsafe impl NSObjectProtocol for QingjianInputController {}
);

/// 数字行与小键盘的键码对应的数字 1–9（ANSI 布局的物理键）。
/// 翻译选中文字最多接受多少个字符：再长既慢又贵，也不是输入法该干的事。
const MAX_TRANSLATE_CHARS: usize = 500;

fn digit_key(key_code: u16) -> Option<usize> {
    Some(match key_code {
        18 | 83 => 1,
        19 | 84 => 2,
        20 | 85 => 3,
        21 | 86 => 4,
        23 | 87 => 5,
        22 | 88 => 6,
        26 | 89 => 7,
        28 | 91 => 8,
        25 | 92 => 9,
        _ => return None,
    })
}

impl QingjianInputController {
    /// 一个按键事件的分发：只管按下；Cmd / Ctrl 组合除 Cmd+左右外一律交给应用；命令键映射成选择器；其余按字符当文本。
    fn dispatch_event(&self, event: &NSEvent, client: TextClient<'_>) -> bool {
        if event.r#type() != NSEventType::KeyDown {
            return false;
        }
        let flags = event.modifierFlags();
        let (command, control, option, shift) = (
            flags.contains(NSEventModifierFlags::Command),
            flags.contains(NSEventModifierFlags::Control),
            flags.contains(NSEventModifierFlags::Option),
            flags.contains(NSEventModifierFlags::Shift),
        );
        let key = event.keyCode();
        let pressed = Modifiers {
            option,
            shift,
            control,
            command,
        };
        // 提示在显示：敲任何键先收掉，键照常处理
        host::with(|h| h.clear_notice());
        // 翻译选中文字进行中：回车 / 空格 / 1 接受，Esc 放弃，其他键放弃后照常交给应用
        if host::with(|h| h.translation.is_some()).unwrap_or(false) {
            return self.handle_translation_review(key, client);
        }
        // 翻译快捷键（不在组句中）：读应用里的选区，交给云端
        let typed = event
            .charactersIgnoringModifiers()
            .map(|c| c.to_string().to_ascii_lowercase());
        let combo = host::with(|h| h.translate_keys).unwrap_or_default();
        if pressed == combo.modifiers
            && typed.as_deref().and_then(|t| t.chars().next()) == Some(combo.key)
            && !host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false)
        {
            return self.translate_selection(client);
        }
        // 修饰键 + 数字：按配置的两组组合上屏第一 / 第二个译词（缺省 ⌥ 与 ⇧⌥）、删候选（缺省 ⇧）。
        // 只在组句中认：不在组句时 ⇧4 就是 `$`，得走下面的标点转换（中文模式出 ￥、⇧6 出 ……、⇧1 出 ！），
        // 以前在这里被截走后原样还给应用，全角转换就没机会做了
        let composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        if composing
            && !pressed.is_empty()
            && let Some(digit) = digit_key(key)
        {
            let (first, second) = host::with(|h| h.translation_keys).unwrap_or_default();
            if pressed == first {
                return self.handle_translation_key(digit, 0, client);
            }
            if pressed == second {
                return self.handle_translation_key(digit, 1, client);
            }
            if pressed == host::with(|h| h.delete_keys).unwrap_or_default() {
                return self.handle_delete_key(digit, client);
            }
        }
        let selector = match key {
            36 | 76 => Some(sel!(insertNewline:)),
            48 if shift => Some(sel!(insertBacktab:)),
            48 => Some(sel!(insertTab:)),
            51 if option => Some(sel!(deleteWordBackward:)),
            51 if command => Some(sel!(deleteToBeginningOfLine:)),
            51 => Some(sel!(deleteBackward:)),
            117 => Some(sel!(deleteForward:)),
            53 => Some(sel!(cancelOperation:)),
            126 => Some(sel!(moveUp:)),
            125 => Some(sel!(moveDown:)),
            123 if command => Some(sel!(moveToLeftEndOfLine:)),
            124 if command => Some(sel!(moveToRightEndOfLine:)),
            123 if option => Some(sel!(moveWordLeft:)),
            124 if option => Some(sel!(moveWordRight:)),
            123 => Some(sel!(moveLeft:)),
            124 => Some(sel!(moveRight:)),
            116 => Some(sel!(pageUp:)),
            121 => Some(sel!(pageDown:)),
            115 => Some(sel!(moveToBeginningOfLine:)),
            119 => Some(sel!(moveToEndOfLine:)),
            _ => None,
        };
        if let Some(selector) = selector {
            return self.handle_command(selector, client);
        }
        if command || control {
            return false;
        }
        match event.characters() {
            Some(text) if !text.is_empty() => self.handle_text(&text.to_string(), client),
            _ => false,
        }
    }

    /// Option+数字：上屏当前页第几个候选的译文（学习和拼音消耗与选那个候选一样）。
    /// 不在组句中时不管；候选没有译文就吞掉按键不动，免得 ¡™£ 进应用。
    /// 翻译应用里选中的文字：云服务关着、密码框、没有选区都不动（键交回应用）。
    fn translate_selection(&self, client: TextClient<'_>) -> bool {
        if !host::with(|h| h.engine.prediction_enabled()).unwrap_or(false) {
            tracing::info!("云服务没开，翻译快捷键不生效");
            return false;
        }
        if secure_input::enabled() {
            tracing::debug!("Secure Input 中，不翻译");
            return false;
        }
        let Some((text, range)) = client.selected_text(MAX_TRANSLATE_CHARS) else {
            // 分不清是没选还是应用不给读（不少 Electron 应用不支持），两种情况都提示一下，键吞掉
            tracing::debug!("没有选中的文字，或应用不支持读选区");
            let anchor = client.caret_rect();
            host::with(|h| {
                h.show_notice(
                    "没有选中的文字，或这个应用不支持读取选区（最多 500 字）",
                    anchor,
                )
            });
            return true;
        };
        // 光标位置先在借用之外取好：取的过程会等应用回话，期间别的 IMK 回调可能重入
        let anchor = client.caret_rect();
        let sent = host::with(|h| {
            h.anchor = anchor;
            h.engine.request_translation(&text).is_some()
        })
        .unwrap_or(false);
        if !sent {
            return false;
        }
        tracing::debug!(chars = text.chars().count(), "翻译选中文字");
        host::with(|h| h.begin_translation(range));
        true
    }

    /// 翻译窗口开着时的按键：回车 / 空格 / 1 用译文替换选区，Esc 放弃；其他键放弃并交回应用。
    fn handle_translation_review(&self, key: u16, client: TextClient<'_>) -> bool {
        let job = host::with(|h| h.translation.clone()).flatten();
        let Some(job) = job else {
            return false;
        };
        match key {
            // 回车 / 小键盘回车 / 空格 / 1：接受（译文还没到时先等）
            36 | 76 | 49 | 18 => {
                if let Some(result) = job.result {
                    tracing::debug!("接受译文");
                    client.replace_range(&result, job.range);
                    host::with(|h| h.end_translation());
                }
                true
            }
            // Esc：放弃
            53 => {
                host::with(|h| h.end_translation());
                true
            }
            _ => {
                host::with(|h| h.end_translation());
                false
            }
        }
    }

    fn handle_translation_key(&self, digit: usize, sense: usize, client: TextClient<'_>) -> bool {
        let composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        if !composing {
            return false;
        }
        let candidate = host::with(|h| {
            h.session
                .index_on_page(digit - 1)
                .and_then(|index| h.session.candidate(index))
        })
        .flatten();
        let text = candidate
            .and_then(|c| host::with(|h| h.engine.commit_translation(&c, sense)).flatten());
        match text {
            Some(text) => {
                tracing::debug!(%text, "commit translation");
                client.insert_text(&text);
                self.refresh(client);
            }
            None => tracing::debug!(digit, sense, "这个候选没有这条译文"),
        }
        true
    }

    /// 修饰键 + 数字（缺省 ⇧）：删掉当前页第几个候选。不在组句中不管；那格没有候选就吞掉按键不动。
    /// 删完重新查一遍（排序会变），结果那句话显示在拼音行右侧，敲下一键就没了。
    fn handle_delete_key(&self, digit: usize, client: TextClient<'_>) -> bool {
        let composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        if !composing {
            return false;
        }
        let Some(message) = host::with(|h| h.forget_candidate(digit - 1)).flatten() else {
            tracing::debug!(digit, "这一格没有候选，没什么可删");
            return true;
        };
        tracing::info!(%message);
        self.refresh(client);
        host::with(|h| h.status = Some(message));
        self.render(client);
        true
    }

    fn handle_text(&self, text: &str, client: TextClient<'_>) -> bool {
        tracing::debug!(%text, "inputText");
        let mut composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        let english = modifiers::caps_lock_on();
        // 终端、编辑器这类应用（`[apps] english_candidates_off`）里英文模式是纯直通
        let english_candidates = english
            && host::with(|h| h.english_candidates_in(client.bundle_identifier().as_deref()))
                .unwrap_or(false);
        // 英文模式组词中 Caps Lock 灭了（或开关关了）：敲的字母先原样上屏，别把它们当拼音
        if composing
            && !english_candidates
            && host::with(|h| h.engine.english_mode()).unwrap_or(false)
        {
            self.commit_raw(client);
            composing = false;
        }
        let [byte] = text.as_bytes() else {
            // 多字符文本（如输入法联动、粘贴）：先把当前候选（英文模式下是敲的字母）上屏，再交给应用
            if composing {
                if host::with(|h| h.engine.english_mode()).unwrap_or(false) {
                    self.commit_raw(client);
                } else {
                    self.commit_highlighted(client);
                }
            }
            return false;
        };
        let c = char::from(*byte);
        host::with(|h| h.indicator.update());
        // 缓冲区为空时敲 ? 先进问字模式，中英文模式都行：后面跟字母就是在问字，跟别的键就还原成问号
        if !composing && c == QUESTION_PREFIX {
            host::with(|h| h.engine.push(c));
            self.refresh(client);
            return true;
        }
        let question = composing && host::with(|h| h.engine.question_mode()).unwrap_or(false);
        // 英文模式下问字：Caps Lock 让字母以大写送来，按小写收进问题
        let c = if question && english && c.is_ascii_uppercase() {
            c.to_ascii_lowercase()
        } else {
            c
        };
        host::with(|h| h.engine.set_english_mode(english_candidates && !question));
        // Caps Lock 亮着 = 英文模式：不组句、不转标点，字母默认小写、按住 Shift 才大写
        if english && !question {
            // Caps Lock 亮着时 macOS 不管按没按 Shift 送来的都是大写，只能读 Shift 状态：按着才大写
            let letter = if modifiers::shift_down() {
                c.to_ascii_uppercase()
            } else {
                c.to_ascii_lowercase()
            };
            if !english_candidates {
                if composing {
                    self.commit_raw(client);
                }
                if c.is_ascii_alphabetic() {
                    client.insert_text(&letter.to_string());
                    host::with(|h| h.engine.note_passthrough(letter));
                    return true;
                }
                host::with(|h| h.engine.note_passthrough(c));
                return false;
            }
            // 英文候选：字母（以及组词中的数字、_ ' -）进缓冲区，候选来自英文词表；
            // 空格、回车、标点先把敲的字母原样上屏再交给应用，数字键照常是数字。
            // 选词靠 Tab 和方向键；用方向键动过高亮之后空格也选那个词（再把空格交给应用），
            // 没动过的空格还是原样上屏——不选词时它和纯直通完全一样，打 kubectl 这类词表没有的词不会被补全替换
            if c.is_ascii_alphabetic()
                || (composing && (c.is_ascii_digit() || matches!(c, '_' | '\'' | '-')))
            {
                host::with(|h| h.engine.push(letter));
                self.refresh(client);
                return true;
            }
            if composing {
                let navigated = host::with(|h| h.session.navigated).unwrap_or(false);
                if c == ' ' && navigated {
                    self.commit_highlighted(client);
                } else {
                    self.commit_raw(client);
                }
            }
            host::with(|h| h.engine.note_passthrough(c));
            return false;
        }
        // 表达式模式（v 开头）：数字与运算符进缓冲区，不当选词 / 翻页键
        let expression = composing && host::with(|h| h.engine.expression_mode()).unwrap_or(false);
        // 英文直输段（缓冲区里已有 `-` 这类字符）：可见字符一律追加，空格 / 回车整段原样上屏
        let raw = composing && host::with(|h| h.engine.raw_mode()).unwrap_or(false);
        // 组句中敲 `-`：进入英文直输段（`no-way`），不再当翻页键；翻页键见配置 `[general] page_keys`
        let hyphen = composing && !question && c == '-';
        // 问字模式下敲的还可能是码点（`u4e00`、`u+1f600`）：数字与 `+` 进缓冲区而不是选词
        let unicode = question && host::with(|h| h.engine.unicode_entry()).unwrap_or(false);
        // 微软 / 搜狗双拼的 `;` 是 ing 键：末尾有落单声母时进缓冲区，其他时候还是标点
        let semicolon =
            composing && c == ';' && host::with(|h| h.engine.takes_semicolon()).unwrap_or(false);
        let (page_previous, page_next) =
            host::with(|h| h.page_keys).unwrap_or(qingjian_platform::DEFAULT_PAGE_KEYS);
        // 组句中敲半角标点：进缓冲区，整段成为英文直输段（`hello,` `dui'ma?`），中文模式下也能打带标点的英文；
        // 翻页键除外；⇧+数字（! @ # …）在前面已被删候选 / 译词键截走
        let punctuation = composing
            && !question
            && !expression
            && c.is_ascii_punctuation()
            && c != page_previous
            && c != page_next;
        if c.is_ascii_lowercase()
            || (composing && c == '\'')
            || semicolon
            || (expression && qingjian_core::shortcut::is_expression_char(c))
            || (raw && c.is_ascii_graphic())
            || (unicode && (c.is_ascii_digit() || c == '+'))
            || hyphen
            || punctuation
        {
            host::with(|h| h.engine.push(c));
            self.refresh(client);
            return true;
        }
        // 直输段里的空格：整段原样上屏，空格本身也交给应用（`hello, world` 里的空格要在）
        if raw && c == ' ' {
            self.commit_highlighted(client);
            host::with(|h| h.engine.note_passthrough(c));
            return false;
        }
        if composing && self.restore_bare_question(client) {
            // 空格只是「把这个 ? 上屏」，不再多打一个空格；其他键按非组句状态继续处理
            if c == ' ' {
                return true;
            }
            return self.handle_text(text, client);
        }
        // 按住 Shift 打的大写字母：临时打英文，先把拼音原样上屏，再把字母交给应用
        if c.is_ascii_uppercase() {
            if composing {
                self.commit_raw(client);
            }
            host::with(|h| h.engine.note_passthrough(c));
            return false;
        }
        if composing {
            match c {
                ' ' => return self.commit_highlighted(client),
                '1'..='9' => {
                    let offset = usize::from(*byte - b'1');
                    let (index, cells) = host::with(|h| {
                        (
                            h.session.index_on_page(offset),
                            h.session.page_cells().len(),
                        )
                    })
                    .unwrap_or((None, 0));
                    return match index {
                        Some(index) => self.commit_index(index, client),
                        // 云端词还没到的占位格：数字键按了不算，免得结果一到就选错
                        None if offset < cells => true,
                        None => self.commit_raw(client),
                    };
                }
                c if c == page_previous => return self.turn_page(-1, client),
                c if c == page_next => return self.turn_page(1, client),
                // 其他字符：把当前高亮候选上屏，再按非组句状态处理这个字符
                _ => {
                    self.commit_highlighted(client);
                }
            }
        }
        // 中文模式下的全角标点；转不了的（数字、字母以外的其他键）原样交给应用
        match host::with(|h| h.engine.punctuate(c)).flatten() {
            Some(full_width) => {
                client.insert_text(full_width);
                true
            }
            None => {
                host::with(|h| h.engine.note_passthrough(c));
                false
            }
        }
    }

    /// 组句期间所有编辑动作都由我们接管；不认识的一律吞掉，否则应用会动光标、丢 marked text。
    fn handle_command(&self, selector: Sel, client: TextClient<'_>) -> bool {
        tracing::debug!(selector = %selector, "didCommandBySelector");
        let composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        if !composing {
            // 删的是应用里的文字：刚上屏的词被整个删掉是「选错了」的信号，Engine 记着；
            // 按词 / 按行删的数不清删了几个字，撤销的账就不记了
            if selector == sel!(deleteBackward:) {
                host::with(|h| h.engine.note_backspace());
            } else if selector == sel!(deleteWordBackward:)
                || selector == sel!(deleteToBeginningOfLine:)
            {
                host::with(|h| h.engine.break_chain());
            }
            return false;
        }
        if selector == sel!(deleteBackward:) {
            host::with(|h| h.engine.backspace());
            self.refresh(client);
        } else if selector == sel!(deleteWordBackward:) {
            // ⌥⌫：删光标前一个音节
            host::with(|h| h.engine.delete_syllable_backward());
            self.refresh(client);
        } else if selector == sel!(deleteToBeginningOfLine:) {
            // ⌘⌫：删光标前的全部拼音
            host::with(|h| h.engine.delete_to_start());
            self.refresh(client);
        } else if selector != sel!(cancelOperation:)
            && selector != sel!(complete:)
            && self.restore_bare_question(client)
        {
            // 只有一个 ? 时按了回车：回车就是「把这个 ? 上屏」，吞掉，否则聊天框里会连消息一起发出去；
            // 方向键等其他键还原后交给应用
            return selector == sel!(insertNewline:);
        } else if selector == sel!(insertNewline:) {
            self.commit_raw(client);
        } else if selector == sel!(cancelOperation:) || selector == sel!(complete:) {
            // TextEdit 等应用把 Esc 绑成 complete:（自动补全），也当作取消
            host::with(|h| {
                h.engine.clear();
                h.cancel_prediction();
            });
            self.refresh(client);
        } else if selector == sel!(insertTab:) {
            // 英文模式 Tab 选中高亮的词；中文模式有整句补全时接受它，否则翻页
            if host::with(|h| h.engine.english_mode()).unwrap_or(false) {
                self.commit_highlighted(client);
            } else if !self.accept_sentence(client) {
                self.turn_page(1, client);
            }
        } else if selector == sel!(deleteForward:) {
            host::with(|h| h.engine.delete_forward());
            self.refresh(client);
        } else if selector == sel!(moveDown:) {
            self.move_highlight(1, client);
        } else if selector == sel!(moveUp:) {
            self.move_highlight(-1, client);
        } else if selector == sel!(moveLeft:) {
            host::with(|h| h.engine.move_cursor_left());
            self.refresh(client);
        } else if selector == sel!(moveRight:) {
            host::with(|h| h.engine.move_cursor_right());
            self.refresh(client);
        } else if selector == sel!(moveWordLeft:) {
            // ⌥←：光标往左跳一个音节
            host::with(|h| h.engine.move_cursor_syllable_left());
            self.refresh(client);
        } else if selector == sel!(moveWordRight:) {
            // ⌥→：光标往右跳一个音节
            host::with(|h| h.engine.move_cursor_syllable_right());
            self.refresh(client);
        } else if selector == sel!(moveToBeginningOfLine:) || selector == sel!(moveToLeftEndOfLine:)
        {
            host::with(|h| h.engine.move_cursor_home());
            self.refresh(client);
        } else if selector == sel!(moveToEndOfLine:) || selector == sel!(moveToRightEndOfLine:) {
            host::with(|h| h.engine.move_cursor_end());
            self.refresh(client);
        } else if selector == sel!(pageDown:) || selector == sel!(scrollPageDown:) {
            self.turn_page(1, client);
        } else if selector == sel!(pageUp:)
            || selector == sel!(scrollPageUp:)
            || selector == sel!(insertBacktab:)
        {
            self.turn_page(-1, client);
        }
        true
    }

    /// 按当前缓冲区重新查候选、更新 marked text，回到第一页并重画候选窗口。
    fn refresh(&self, client: TextClient<'_>) {
        let Some((marked, cursor, inline)) = host::with(|h| {
            // 查询失败（整段切不动）时退回显示原始字母
            let mut marked = h.engine.composition().text().to_owned();
            let mut cursor = h.engine.composition().cursor();
            let mut preedit = Preedit::plain(&marked, cursor);
            let candidates = h
                .engine
                .query()
                .map(|mut query| {
                    h.engine.annotate(&mut query.candidates);
                    marked = query.marked_text();
                    cursor = query.marked_cursor();
                    preedit = Preedit::from_marked(&query.marked_segments(), cursor);
                    query.candidates.items
                })
                .unwrap_or_default();
            h.reset_session(preedit, candidates);
            (marked, cursor, h.preedit_mode.inline())
        }) else {
            return;
        };
        // 配置成只在候选窗口显示拼音时，应用里不放 marked text（光标位置仍按插入点取）
        if inline {
            client.set_marked_text(&marked, cursor);
        } else {
            client.set_marked_text("", 0);
        }
        // 先发联想再画：发出去就留好云端槽位，画出来的第一帧本地候选就已经在最终位置
        if !marked.is_empty() {
            let candidates = host::with(|h| h.session.layout.local().to_vec()).unwrap_or_default();
            self.request_prediction(client, &candidates);
        }
        self.render(client);
    }

    /// 缓冲区里只有一个 `?` 而用户按了别的键：把它还原成问号上屏（中文模式全角、英文模式半角）、清空缓冲区。
    /// 返回是否发生了还原。
    fn restore_bare_question(&self, client: TextClient<'_>) -> bool {
        let english = modifiers::caps_lock_on();
        let restored = host::with(|h| {
            if !h.engine.bare_question() {
                return None;
            }
            h.engine.clear();
            h.cancel_prediction();
            if english {
                h.engine.note_passthrough(QUESTION_PREFIX);
                return Some(QUESTION_PREFIX.to_string());
            }
            h.engine.punctuate(QUESTION_PREFIX).map(str::to_owned)
        })
        .flatten();
        let Some(mark) = restored else {
            return false;
        };
        client.insert_text(&mark);
        self.refresh(client);
        true
    }

    /// 记下光标位置并按会话状态重画候选窗口。
    fn render(&self, client: TextClient<'_>) {
        let anchor = client.caret_rect();
        host::with(|h| {
            h.anchor = anchor;
            h.render();
        });
    }

    /// 发一次联想请求。Secure Input 里绝不发；没接联想器时是空操作。
    ///
    /// 读上下文要等应用回话，这段时间 IMK 可能把 `deactivateServer:` 之类的回调插进来，
    /// 所以分两次借 Host：先拿策略、放开借用去读、再借回来发请求。
    fn request_prediction(&self, client: TextClient<'_>, candidates: &[Candidate]) {
        let policy = host::with(|h| {
            if !h.engine.prediction_enabled() {
                return None;
            }
            if secure_input::enabled() {
                tracing::debug!("Secure Input 中，不联想");
                h.cancel_prediction();
                return None;
            }
            Some(h.engine.prediction_policy())
        })
        .flatten();
        let Some(policy) = policy else {
            return;
        };
        let surrounding = client.surrounding_text(policy.before, policy.after);
        host::with(|h| {
            tracing::debug!(
                has_context = surrounding.is_some(),
                pinyin = h.engine.composition().scope(),
                "联想请求"
            );
            match h.engine.request_prediction(surrounding, candidates) {
                Some(_) => h.await_prediction(),
                None => h.cancel_prediction(),
            }
        });
    }

    /// 接受组句中的整句补全：作用域内的拼音作废，句子上屏。没有补全返回 false。
    fn accept_sentence(&self, client: TextClient<'_>) -> bool {
        let Some(text) = host::with(|h| h.sentence.take()).flatten() else {
            return false;
        };
        host::with(|h| h.engine.accept_prediction(&text));
        tracing::debug!(%text, "接受整句补全");
        client.insert_text(&text);
        self.refresh(client);
        true
    }

    /// 高亮上下移动，越过页边自动翻页。
    fn move_highlight(&self, delta: isize, client: TextClient<'_>) {
        if host::with(|h| h.session.move_highlight(delta)).unwrap_or(false) {
            self.render(client);
        }
    }

    /// 翻页，高亮落到新页第一项。已在首页 / 末页时不动。
    fn turn_page(&self, delta: isize, client: TextClient<'_>) -> bool {
        if host::with(|h| h.session.turn_page(delta)).unwrap_or(false) {
            self.render(client);
        }
        true
    }

    fn commit_highlighted(&self, client: TextClient<'_>) -> bool {
        let index = host::with(|h| h.session.highlighted).unwrap_or(0);
        self.commit_index(index, client)
    }

    /// 上屏第 `index` 个候选；没有候选时上屏拼音本身。上屏后剩余拼音继续组句。
    fn commit_index(&self, index: usize, client: TextClient<'_>) -> bool {
        let candidate = host::with(|h| h.session.candidate(index)).flatten();
        let Some(candidate) = candidate else {
            return self.commit_raw(client);
        };
        let Some(text) = host::with(|h| h.engine.commit(&candidate)) else {
            return false;
        };
        tracing::debug!(%text, "commit");
        client.insert_text(&text);
        self.refresh(client);
        true
    }

    /// 把拼音原样上屏并清空。缓冲区为空时返回 false。
    fn commit_raw(&self, client: TextClient<'_>) -> bool {
        let Some(raw) = host::with(|h| h.engine.take_raw()) else {
            return false;
        };
        if raw.is_empty() {
            return false;
        }
        tracing::debug!(%raw, "commit raw");
        client.insert_text(&raw);
        self.refresh(client);
        true
    }
}
