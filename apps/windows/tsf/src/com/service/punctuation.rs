//! 标点切换键的登记与触发：Server 下发配置，避免读取宿主不可访问的用户目录。

use qingjian_platform::KeyCombo;
use qingjian_platform::protocol::IndicatorCommand;
use windows::Win32::UI::TextServices::ITfKeystrokeMgr;
use windows::core::Interface;

use super::TextService_Impl;
use crate::com::key::preserved;
use crate::com::log::log;

impl TextService_Impl {
    pub(super) fn sync_punctuation_key(&self, combo: Option<KeyCombo>) {
        if combo == self.punctuation_combo.get() {
            return;
        }
        let Some(manager) = self.thread_mgr.borrow().clone() else {
            return;
        };
        let Ok(keys) = manager.cast::<ITfKeystrokeMgr>() else {
            return;
        };
        if let Some(previous) = self.punctuation_combo.take() {
            preserved::unregister_punctuation(&keys, previous);
        }
        if let Some(combo) = combo {
            match preserved::register_punctuation(&keys, self.client_id.get(), combo) {
                Ok(()) => self.punctuation_combo.set(Some(combo)),
                Err(error) => log(&format!("登记标点切换快捷键失败: {error}")),
            }
        }
    }

    pub(super) fn toggle_punctuation(&self) -> bool {
        self.key_tap.cancel();
        if !self.ensure_connected() {
            return false;
        }
        let result = self
            .engine
            .borrow_mut()
            .as_mut()
            .map(|client| client.indicator(IndicatorCommand::TogglePunctuation));
        match result {
            Some(Ok(())) => true,
            Some(Err(error)) => {
                log(&format!("切换标点失败: {error}"));
                self.disconnect();
                false
            }
            None => false,
        }
    }
}
