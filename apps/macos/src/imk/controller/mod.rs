//! IMK 输入控制器：每个输入会话（每个应用的文本框）一个实例。
//!
//! 只做两件事：把按键翻译成 Engine 的调用，把 Engine 返回的候选交给候选窗口。
//! **这里不允许出现排序、词库或翻译逻辑。** 会话状态（候选、高亮、页码）在 [`crate::host::Session`]。

use objc2::rc::{Allocated, Retained};
use objc2::runtime::{AnyObject, Sel};
use objc2::{define_class, msg_send, sel};
use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType, NSMenu};
use objc2_foundation::{NSObjectProtocol, NSString};
use objc2_input_method_kit::{IMKInputController, IMKServer};
use qingjian_core::candidate::arrows::Focus;
use qingjian_core::{Candidate, QUESTION_PREFIX};
use qingjian_platform::Modifiers;

use super::{TextClient, catch_panic, modifiers, recover_from_panic, secure_input};
use crate::candidates::Preedit;
use crate::host;
use crate::menubar;

mod candidate_editor;
mod click;
mod command;
mod commit;
mod display;
mod panel;
mod text;
mod translate;

pub use click::{click, menu_items, menu_pick, panel_mouse};

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
            self.set_active();
            let done = catch_panic("activateServer", || {
                // 会话级把键盘布局覆盖为 ABC（QWERTY）：系统会沿用「上一次使用的拉丁键盘布局」，
                // 从德语 QWERTZ 等布局切入时 z/y 互换（#577）。这是 IMKTextInput 协议方法，
                // 只作用于本输入法收到的按键事件，不改用户的全局布局；objc2-input-method-kit
                // 未生成该协议绑定，与 TextClient 一样用裸 msg_send!。
                if let Some(sender) = sender {
                    let layout = NSString::from_str("com.apple.keylayout.ABC");
                    let _: () =
                        unsafe { msg_send![sender, overrideKeyboardWithKeyboardNamed: &*layout] };
                }
                // 用户要往 [apps] 里加应用时，从这条日志抄 bundle identifier
                let bundle = sender.and_then(|s| TextClient::new(s).bundle_identifier());
                if let Some(bundle) = &bundle {
                    tracing::debug!(%bundle, "当前应用");
                }
                host::with(|h| {
                    h.engine.set_application(bundle);
                    h.engine.enter_input_box();
                    h.refresh_text_replacements();
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
            match menubar::action_from_sender(sender) {
                Some(menubar::MenuAction::OpenSymbolPanel) => self.open_panel_from_menu(),
                Some(action) => {
                    host::with(|h| h.perform(action));
                }
                None => {}
            }
        }

        #[unsafe(method(deactivateServer:))]
        fn deactivate_server(&self, sender: Option<&AnyObject>) {
            tracing::info!("deactivateServer");
            self.clear_active();
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

/// 翻译选中文字最多接受多少个字符：再长既慢又贵，也不是输入法该干的事。
const MAX_TRANSLATE_CHARS: usize = 500;

/// 给本地模型看的光标前文最多读多少字符：知意英文要 192，含章由 Engine 再按它的前文长度截。
const RESCORE_LOOKBACK: usize = 192;

/// 登录 / 锁屏窗口的 bundle identifier。
const LOGIN_WINDOW: &str = "com.apple.loginwindow";

/// 数字行与小键盘的键码对应的数字 1–9（ANSI 布局的物理键）。
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
    /// 登录 / 锁屏窗口：输入源菜单里没有青简，loginwindow 却照样激活它，按键一律交还系统。
    ///
    /// TODO(#190): 临时防护。现象是开机登录界面打不进模式键（u / i），推断为按键进了青简的组句；
    /// 日志只证实 loginwindow 会激活青简，按键是否真的送来没有复现（开 FileVault 的机器进不到这个界面）。
    /// 找到按键送进来的条件后改成针对它的判断，并确认别的系统界面有没有同样的情况。
    fn in_login_window(&self) -> bool {
        host::with(|h| h.engine.application() == Some(LOGIN_WINDOW)).unwrap_or(false)
    }

    /// 一个按键事件的分发：只管按下；Cmd / Ctrl 组合除 Cmd+左右外一律交给应用；命令键映射成选择器；其余按字符当文本。
    fn dispatch_event(&self, event: &NSEvent, client: TextClient<'_>) -> bool {
        if event.r#type() != NSEventType::KeyDown || self.in_login_window() {
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
        // 候选右键菜单开着：Esc 只关菜单、不清输入；别的键关菜单后照常处理
        if host::with(|h| h.window.menu_open()).unwrap_or(false) {
            host::with(|h| h.window.close_menu());
            if key == 53 {
                return true;
            }
        }
        // 翻译选中文字进行中：回车 / 空格 / 1 接受，Esc 放弃，其他键放弃后照常交给应用
        if host::with(|h| h.translation.is_some()).unwrap_or(false) {
            return self.handle_translation_review(key, client);
        }
        // 符号面板开着：按键先归面板；敲字母时面板收起、按键照常往下走
        if host::with(|h| h.engine.symbol_panel().is_some()).unwrap_or(false)
            && let Some(consumed) = self.handle_panel_key(event, key, pressed, client)
        {
            return consumed;
        }
        // 高亮还在第一个时按 ↑ 打开符号面板；长按连发的不算，免得往回翻过头弹出来。
        // 方向键分两区时（arrows::route）候选里的 ↑ 是回拼音行，要在拼音行里再按一次 ↑
        if key == 126
            && pressed.is_empty()
            && !event.isARepeat()
            && host::with(|h| {
                !h.engine.composition().is_empty()
                    && (!h.two_area_arrows() || h.session.focus == Focus::Pinyin)
            })
            .unwrap_or(false)
            && self.open_panel(client)
        {
            return true;
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
        // 按需联想键（`[shortcut] predict`，缺省 ⌃⌥J）：组句中立刻请求一次，结果排在第一页最前面
        let predict = host::with(|h| h.predict_keys).unwrap_or_default();
        if pressed == predict.modifiers
            && typed.as_deref().and_then(|t| t.chars().next()) == Some(predict.key)
            && host::with(|h| !h.engine.composition().is_empty() && h.engine.prediction_enabled())
                .unwrap_or(false)
        {
            let candidates = host::with(|h| h.session.layout.local().to_vec()).unwrap_or_default();
            self.request_prediction(client, &candidates, true);
            return true;
        }
        // 标点切换键（`[shortcut] toggle_punctuation`，设置程序里勾上是 Ctrl + .）：切的是中文模式的标点，
        // 英文模式一律半角，这时键照常交给应用
        if let Some(combo) = host::with(|h| h.punctuation_keys).flatten()
            && pressed == combo.modifiers
            && typed.as_deref().and_then(|t| t.chars().next()) == Some(combo.key)
            && !modifiers::caps_lock_on()
        {
            host::with(|h| h.toggle_punctuation());
            return true;
        }
        // 修饰键 + 数字：按配置的两组组合上屏第一 / 第二个译词（缺省 ⌥ 与 ⇧⌥）。
        // 只在组句中认：不在组句时 ⇧4 就是 `$`，得走下面的标点转换（中文模式出 ￥、⇧6 出 ……、⇧1 出 ！），
        // 以前在这里被截走后原样还给应用，全角转换就没机会做了。
        // 表达式模式（`v2^3`）里 ⇧+数字打的是 `^ * ( )`，不当快捷键
        let composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        let expression = composing && host::with(|h| h.engine.expression_mode()).unwrap_or(false);
        if composing
            && !expression
            && !pressed.is_empty()
            && let Some(digit) = digit_key(key)
        {
            let (first, second) = host::with(|h| h.translation_keys).unwrap_or_default();
            if pressed == first {
                tracing::info!(digit, sense = 0, "译词快捷键命中");
                return self.handle_translation_key(digit, 0, client);
            }
            if pressed == second {
                tracing::info!(digit, sense = 1, "译词快捷键命中");
                return self.handle_translation_key(digit, 1, client);
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
}
