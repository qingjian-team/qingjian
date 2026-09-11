//! 中 / 英模式：切模式先把组着的内容落定；指示器走语言栏按钮 + 转换模式 compartment，并推给 Server 的状态条；
//! 用户点任务栏中 / 英时由 compartment 回调反向同步。

use windows::Win32::UI::TextServices::ITfLangBarItemMgr;
use windows::core::Interface;

use super::TextService_Impl;
use crate::com::log::log;
use crate::com::mode::{self, ModeButton, conversion};

impl TextService_Impl {
    /// 切模式：先把组着的内容原样落定，再刷指示器。
    pub(super) fn set_english_mode(&self, english: bool) {
        self.commit_pending();
        self.mode_state.set_english(english);
        self.refresh_mode_indicator();
        log(if english {
            "切到英文模式"
        } else {
            "切到中文模式"
        });
    }

    /// 语言栏按钮换图标、写转换模式 compartment、把模式推给 Server（悬浮状态条）。
    pub(super) fn refresh_mode_indicator(&self) {
        let english = self.mode_state.english();
        self.mode_state.notify();
        if let Some(thread_mgr) = self.thread_mgr.borrow().as_ref() {
            mode::set_indicator(thread_mgr, self.client_id.get(), english);
        }
        if let Some(client) = self.engine.borrow_mut().as_mut()
            && let Err(error) = client.mode_changed(english)
        {
            log(&format!("上报中英模式失败: {error}"));
        }
    }

    pub(super) fn advise_conversion_sink(&self) {
        let Some(thread_mgr) = self.thread_mgr.borrow().clone() else {
            return;
        };
        match conversion::advise(&thread_mgr) {
            Ok(advice) => *self.conversion_sink.borrow_mut() = Some(advice),
            Err(error) => log(&format!("监听转换模式失败: {error}")),
        }
    }

    pub(super) fn unadvise_conversion_sink(&self) {
        if let Some((source, cookie)) = self.conversion_sink.borrow_mut().take() {
            conversion::unadvise(&source, cookie);
        }
    }

    /// 用户在任务栏点了中 / 英：读回 `NATIVE` 位，与当前不同才切（相同是自己那次写触发的，防回环）。
    pub(super) fn sync_from_conversion_mode(&self) {
        let Some(thread_mgr) = self.thread_mgr.borrow().clone() else {
            return;
        };
        let Ok(compartment) = mode::conversion_compartment(&thread_mgr) else {
            return;
        };
        let english = mode::is_english(&compartment);
        if english != self.mode_state.english() {
            self.set_english_mode(english);
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
