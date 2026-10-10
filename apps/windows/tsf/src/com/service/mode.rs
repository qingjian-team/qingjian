//! 中 / 英模式：全局一份，存在 Server 那边，所有应用共用。
//! 用户在这里切了就报给 Server（[`TextService_Impl::set_english_mode`]）；激活、得到焦点和每隔几拍的轮询
//! 向 Server 取当前模式跟上（[`TextService_Impl::sync_mode_from_server`]）。切模式先把组着的内容落定；
//! 指示器走语言栏按钮 + 转换模式 compartment；用户点任务栏中 / 英由 compartment 回调反向同步。
//! 切换键与内置英文模式开关由 Server 经协议下发（[`TextService_Impl::apply_input_settings`]），DLL 不读配置文件。

use std::time::Instant;

use windows::Win32::UI::TextServices::{ITfKeystrokeMgr, ITfLangBarItemMgr};
use windows::core::Interface;

use qingjian_platform::SwitchKeys;
use qingjian_platform::protocol::InputSettings;

use super::TextService_Impl;
use crate::com::key::preserved;
use crate::com::log::log;
use crate::com::mode::{self, ModeButton, sink};

/// 激活后多久内忽略系统写回的转换模式：msctf 在 TIP 激活后约 200–300 ms 会把 profile 存的
/// 转换模式写回 compartment，不忽略的话它会被当成用户点了任务栏、改掉全局模式。
pub(super) const CONVERSION_RESTORE_GUARD: std::time::Duration =
    std::time::Duration::from_millis(1200);

impl TextService_Impl {
    /// 应用中英模式的两项设置：激活时与配置变更时都走这里。
    ///
    /// 内置英文模式开关在运行中翻转时，语言栏的中 / 英按钮与转换模式回调跟着登记 / 撤掉（激活时由 `Activate`
    /// 自己按开关登记，这里只管激活之后的变化），设置窗口改完不用切走再切回输入法。
    pub(super) fn apply_mode_settings(&self, english_mode: bool, switch_keys: SwitchKeys) {
        let was_enabled = self.mode_state.enabled();
        self.mode_state.set_settings(english_mode, switch_keys);
        // 关掉内置英文模式时立刻回中文，别停在一个再也切不回去的英文状态（Server 那份也跟着回中文）。
        if !english_mode && self.mode_state.english() {
            self.mode_state.set_english(false);
            self.refresh_mode_indicator();
        }
        if was_enabled != english_mode && self.is_active() {
            if english_mode {
                log("打开了内置英文模式：登记中 / 英按钮");
                self.add_lang_bar_item();
                self.advise_mode_sinks();
            } else {
                log("关掉了内置英文模式：撤掉中 / 英按钮");
                self.unadvise_mode_sinks();
                self.remove_lang_bar_item();
            }
        }
        self.sync_switch_preserved_key(switch_keys.ctrl_alt_space);
    }

    /// `Activate` 是否已经走完（[`super::ACTIVE`] 在它末尾才设）。
    fn is_active(&self) -> bool {
        super::ACTIVE.with(|active| active.borrow().is_some())
    }

    /// 应用 Server 下发的按键行为设置：`OpenSession` 的回包给一次，之后每一拍 `SyncMode` 也都带着。
    /// 值没变就什么都不做，所以设置窗口改完在下一拍（约 320 ms）生效，不用切走再切回输入法。
    pub(super) fn apply_input_settings(&self, input: InputSettings) {
        let previous = self.input_settings.replace(Some(input));
        if previous == Some(input) {
            return;
        }
        let previous_predict = previous.and_then(|input| input.predict);
        if previous_predict != input.predict
            && let Some(thread_mgr) = self.thread_mgr.borrow().clone()
            && let Ok(keystroke) = thread_mgr.cast::<ITfKeystrokeMgr>()
        {
            if let Some(combo) = previous_predict {
                preserved::unregister(&keystroke, &preserved::GUID_PREDICT, combo);
            }
            if let Some(combo) = input.predict {
                match preserved::register(
                    &keystroke,
                    self.client_id.get(),
                    &preserved::GUID_PREDICT,
                    combo,
                    "AI 联想（青简）",
                ) {
                    Ok(()) => log(&format!("AI 联想快捷键已登记为保留键: {combo}")),
                    Err(error) => log(&format!("登记 AI 联想快捷键失败: {error}")),
                }
            }
        }
        log(&format!(
            "按键行为设置：中英切换键 {}，内置英文模式 {}，Shift 字母进组句 {}，英文候选 {}",
            input.switch_mode.describe(),
            input.english_mode,
            input.shift_letter_compose,
            input.english_candidates
        ));
        self.apply_mode_settings(input.english_mode, input.switch_mode);
    }

    /// Ctrl + Alt + Space 是组合键、走 TSF 保留键（与「翻译选中文字」同一套）；没勾就撤掉登记，免得白占着。
    fn sync_switch_preserved_key(&self, want: bool) {
        if want == self.switch_preserved.get() {
            return;
        }
        let Some(thread_mgr) = self.thread_mgr.borrow().clone() else {
            return;
        };
        let Ok(keystroke) = thread_mgr.cast::<ITfKeystrokeMgr>() else {
            return;
        };
        if want {
            match preserved::register_switch_mode(&keystroke, self.client_id.get()) {
                Ok(()) => {
                    self.switch_preserved.set(true);
                    log("中英切换键 Ctrl + Alt + Space 已登记为保留键");
                }
                Err(error) => log(&format!("登记 Ctrl + Alt + Space 切换键失败: {error}")),
            }
        } else {
            preserved::unregister_switch_mode(&keystroke);
            self.switch_preserved.set(false);
        }
    }

    /// 停用时撤掉 Ctrl + Alt + Space 的保留键登记。
    pub(super) fn drop_switch_preserved_key(&self, keystroke: &ITfKeystrokeMgr) {
        if self.switch_preserved.replace(false) {
            preserved::unregister_switch_mode(keystroke);
        }
    }

    /// 用户在这个应用里切了模式（切换键、语言栏按钮、右键菜单）：改状态、刷指示器，报给 Server 成为全局模式。
    pub(super) fn set_english_mode(&self, english: bool) {
        if self.switch_mode(english) {
            self.refresh_mode_indicator();
            self.report_mode();
        }
    }

    /// 跟上 Server 的全局模式（别的应用切过、点了悬浮状态条）：改状态、刷指示器，不再回报。
    pub(super) fn adopt_mode(&self, english: bool) {
        if english != self.mode_state.english() && self.switch_mode(english) {
            self.refresh_mode_indicator();
        }
    }

    /// 向 Server 取一次全局模式跟上，顺路取回按键行为设置与菜单状态；没连着就什么都不做。
    pub(super) fn sync_mode_from_server(&self) {
        let reply = self
            .engine
            .borrow_mut()
            .as_mut()
            .map(|client| client.sync_mode());
        match reply {
            Some(Ok(reply)) => {
                self.apply_input_settings(reply.input);
                self.indicator_state.set(reply.indicator);
                if let Some(english) = reply.english {
                    self.adopt_mode(english);
                }
            }
            Some(Err(error)) => {
                log(&format!("同步中英模式失败，断开，下一键重连: {error}"));
                self.disconnect();
            }
            None => {}
        }
    }

    /// 用户点了任务栏中 / 英、按了系统 Ctrl + Space：只更新按钮，不回写 compartment（在它自己的 `OnChange` 里写会报
    /// 0x8000FFFF），报给 Server。
    fn follow_system_mode(&self, english: bool) {
        if self.switch_mode(english) {
            self.mode_state.notify();
            self.report_mode();
        }
    }

    /// 两条路共用：先把组着的内容原样落定，再改状态。返回是否真的改了。
    ///
    /// 配置关掉了内置英文模式时什么都不做——切换键、语言栏按钮、悬浮状态条、任务栏转换模式四条入口
    /// 都汇到这里，一处拦住就再也进不了英文模式（见 issue #81）。
    fn switch_mode(&self, english: bool) -> bool {
        if !self.mode_state.enabled() {
            if english {
                log("内置英文模式已关闭，忽略切到英文");
            }
            return false;
        }
        self.commit_pending();
        self.mode_state.set_english(english);
        log(if english {
            "切到英文模式"
        } else {
            "切到中文模式"
        });
        true
    }

    /// 语言栏按钮换图标，写转换模式与输入法开关两条 compartment：开关跟着模式走（中文开、英文关），
    /// 系统的 Ctrl + Space 翻的就是它，对上了才能一按就切。
    pub(super) fn refresh_mode_indicator(&self) {
        self.mode_state.notify();
        let english = self.mode_state.english();
        if let Some(thread_mgr) = self.thread_mgr.borrow().as_ref() {
            mode::set_indicator(thread_mgr, self.client_id.get(), english);
            mode::set_keyboard_open(thread_mgr, self.client_id.get(), !english);
        }
    }

    /// 本线程得到 / 失去键盘焦点。得到：补连接，跟上全局模式（别的应用里可能刚切过）。
    pub(super) fn set_thread_focus(&self, foreground: bool) {
        if self.shared.foreground() == foreground {
            return;
        }
        self.shared.set_foreground(foreground);
        // AdviseKeyEventSink 会在 Activate 尚未准备好 thread_mgr 时同步获焦；连接由 Activate 随后补上。
        if foreground && self.thread_mgr.borrow().is_some() {
            self.ensure_connected();
            self.sync_mode_from_server();
        }
    }

    /// 把用户切出的模式报给 Server，成为全局模式。
    pub(super) fn report_mode(&self) {
        let english = self.mode_state.english();
        if let Some(client) = self.engine.borrow_mut().as_mut()
            && let Err(error) = client.mode_changed(english)
        {
            log(&format!("上报中英模式失败: {error}"));
        }
    }

    /// 挂上「转换模式」与「输入法开 / 关」的回调。
    pub(super) fn advise_mode_sinks(&self) {
        let Some(thread_mgr) = self.thread_mgr.borrow().clone() else {
            return;
        };
        *self.mode_sinks.borrow_mut() = sink::advise(&thread_mgr);
    }

    pub(super) fn unadvise_mode_sinks(&self) {
        sink::unadvise(std::mem::take(&mut *self.mode_sinks.borrow_mut()));
    }

    /// 用户在任务栏点了中 / 英：读回 `NATIVE` 位，与当前不同才跟着切（相同是自己那次写触发的，防回环）。
    ///
    /// 激活后的最初一瞬不算：那时 msctf 在把 profile 存的转换模式写回来，不是用户操作。
    pub(super) fn sync_from_conversion_mode(&self) {
        if let Some(until) = self.conversion_guard_until.get()
            && Instant::now() < until
        {
            log("激活后忽略一次系统写回的转换模式（msctf 的 profile 恢复，不是用户操作）");
            return;
        }
        let Some(thread_mgr) = self.thread_mgr.borrow().clone() else {
            return;
        };
        let Ok(compartment) = mode::conversion_compartment(&thread_mgr) else {
            return;
        };
        let english = mode::is_english(&compartment);
        if english != self.mode_state.english() {
            log(&format!(
                "转换模式变了（任务栏 / 系统快捷键），english={english}"
            ));
            self.follow_system_mode(english);
        }
    }

    /// 系统的「输入法/非输入法切换」（缺省 Ctrl + Space）翻了输入法开关：关 = 英文、开 = 中文，与微软拼音一致。
    /// 这个热键的 Space 被系统截走，按着的 Ctrl 抬起时别再当成单击。
    pub(super) fn sync_from_keyboard_open(&self) {
        self.key_tap.cancel();
        let Some(thread_mgr) = self.thread_mgr.borrow().clone() else {
            return;
        };
        let Ok(compartment) = mode::openclose_compartment(&thread_mgr) else {
            return;
        };
        let english = !mode::is_keyboard_open(&compartment);
        if english != self.mode_state.english() {
            log(&format!(
                "输入法开关变了（系统 Ctrl + Space），english={english}"
            ));
            self.follow_system_mode(english);
        }
    }

    pub(super) fn add_lang_bar_item(&self) {
        let button = ModeButton::create(self.mode_state.clone());
        if let Some(thread_mgr) = self.thread_mgr.borrow().as_ref() {
            match thread_mgr.cast::<ITfLangBarItemMgr>() {
                Ok(mgr) => {
                    if let Err(error) = unsafe { mgr.AddItem(&button) } {
                        log(&format!("登记中英指示器失败: {error}"));
                    }
                }
                Err(error) => log(&format!("取语言栏管理器失败: {error}")),
            }
        }
        *self.mode_button.borrow_mut() = Some(button);
    }

    pub(super) fn remove_lang_bar_item(&self) {
        if let Some(button) = self.mode_button.borrow_mut().take()
            && let Some(thread_mgr) = self.thread_mgr.borrow().as_ref()
            && let Ok(mgr) = thread_mgr.cast::<ITfLangBarItemMgr>()
        {
            let _ = unsafe { mgr.RemoveItem(&button) };
        }
    }
}
