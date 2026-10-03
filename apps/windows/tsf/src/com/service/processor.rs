//! `ITfTextInputProcessor`：激活时挂击键 sink、登记翻译保留键、连 Server、起轮询定时器、挂 profile /
//! 模式 compartment 回调、登记语言栏按钮；停用按相反顺序撤掉，敲了一半的拼音先原样落定。

use std::time::Instant;

use windows::Win32::UI::TextServices::{
    ITfKeyEventSink, ITfKeystrokeMgr, ITfTextInputProcessor_Impl, ITfThreadMgr,
};
use windows::core::{IUnknownImpl, Interface, Ref, Result};

use qingjian_platform::protocol::InputSettings;

use super::mode::CONVERSION_RESTORE_GUARD;
use super::{ACTIVE, TextService_Impl};
use crate::com::focus;
use crate::com::key::preserved;
use crate::com::log::log;
use crate::com::poll::PollTimer;
use crate::com::profile;

impl ITfTextInputProcessor_Impl for TextService_Impl {
    fn Activate(&self, ptim: Ref<ITfThreadMgr>, tid: u32) -> Result<()> {
        let thread_mgr = ptim.ok()?.clone();
        let keystroke: ITfKeystrokeMgr = thread_mgr.cast()?;
        let sink: ITfKeyEventSink = self.to_interface();
        unsafe { keystroke.AdviseKeyEventSink(tid, &sink, true)? };
        let combo = preserved::load_combo();
        match preserved::register(&keystroke, tid, combo) {
            Ok(()) => {
                self.translate_combo.set(Some(combo));
                log(&format!("翻译选中文字快捷键已登记为保留键: {combo}"));
            }
            Err(error) => log(&format!("登记翻译快捷键失败: {error}")),
        }

        self.client_id.set(tid);
        // 连不上 Server、没定时器都不致命。
        match PollTimer::new(self.engine.clone(), self.shared.clone(), tid) {
            Ok(timer) => *self.poll_timer.borrow_mut() = Some(timer),
            Err(error) => log(&format!("挂云联想轮询定时器失败: {error}")),
        }

        if self.profile_cookie.get().is_none() {
            match profile::advise(&thread_mgr, crate::com::session_id()) {
                Ok(cookie) => self.profile_cookie.set(Some(cookie)),
                Err(error) => log(&format!("监听输入法切换失败: {error}")),
            }
        }
        match focus::advise(&thread_mgr) {
            Ok(advice) => *self.focus_sink.borrow_mut() = Some(advice),
            Err(error) => log(&format!("监听线程焦点失败: {error}")),
        }
        // 激活时线程已有焦点的话，不会再来一次「得到焦点」。
        let focused = unsafe { thread_mgr.IsThreadFocus() }.is_ok_and(|focus| focus.as_bool());
        *self.thread_mgr.borrow_mut() = Some(thread_mgr);
        // 连 Server：它随 `OpenSession` 的回包把按键行为设置带下来，就地应用。那两个值在按键到达之前
        // 就要有，而且应用时要登记 Ctrl + Alt + Space 保留键，所以这一步必须排在 `thread_mgr` 就绪之后。
        self.connect();
        // 只有**没连上** Server 时才用缺省值把模式状态建起来。连上了的话 `connect` 已经应用过 Server 下发的
        // 真实值，这里再应用一次缺省值会把它盖掉（要等下一拍 `SyncMode` 才改回来 —— 关掉内置英文模式的人
        // 每次激活都会先登记上中 / 英按钮，之后也不会撤）。
        if self.engine.borrow().is_none() {
            self.apply_input_settings(InputSettings::default());
        }
        // 模式是全局的：跟上 Server 那份（没连上就先中文）。msctf 激活后还会把上次的转换模式写回来，
        // 那不是用户操作，这段窗口里忽略它（见 `sync_from_conversion_mode`）。
        self.conversion_guard_until
            .set(Some(Instant::now() + CONVERSION_RESTORE_GUARD));
        self.mode_state.set_english(false);
        if focused {
            self.shared.set_foreground(true);
        }
        self.sync_mode_from_server();
        // 顺带把输入法开关对上模式：它在新线程里缺省是关，不对上的话中文模式下第一次按系统 Ctrl + Space 会白按。
        self.refresh_mode_indicator();
        if self.mode_state.enabled() {
            self.add_lang_bar_item();
            // 放在初始写指示器 / 开关之后，别被自己那次写触发。
            self.advise_mode_sinks();
        } else {
            log("配置关掉了内置英文模式：不登记中 / 英按钮，固定中文模式");
        }
        ACTIVE.with(|active| *active.borrow_mut() = Some(self.to_object()));
        log(&format!("青简 TSF 已激活 tid={tid}"));
        Ok(())
    }

    fn Deactivate(&self) -> Result<()> {
        // 先撤回调，之后不再有回调碰本服务。
        ACTIVE.with(|active| active.borrow_mut().take());
        self.unadvise_mode_sinks();
        if let Some((source, cookie)) = self.focus_sink.borrow_mut().take() {
            focus::unadvise(&source, cookie);
        }
        self.remove_lang_bar_item();
        self.poll_timer.borrow_mut().take();
        // 切走输入法时敲了一半的拼音原样落定，再关会话。
        self.commit_pending();
        if let Some(thread_mgr) = self.thread_mgr.borrow_mut().take()
            && let Ok(keystroke) = thread_mgr.cast::<ITfKeystrokeMgr>()
        {
            self.drop_switch_preserved_key(&keystroke);
            if let Some(combo) = self.translate_combo.take() {
                preserved::unregister(&keystroke, combo);
            }
            let _ = unsafe { keystroke.UnadviseKeyEventSink(self.client_id.get()) };
        }
        if let Some(client) = self.engine.borrow_mut().take() {
            let _ = client.close();
        }
        self.pending_digit.borrow_mut().take();
        self.shared.reset();
        self.shared.take_server_stale();
        self.shared.set_foreground(false);
        log("青简 TSF 已停用");
        Ok(())
    }
}
