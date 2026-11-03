//! 设备页：设备名与配对。

use std::net::SocketAddr;

use qingjian_clipboard::ClipSettings;

use super::ClipboardApp;
use crate::link::Pairing;

impl ClipboardApp {
    pub(super) fn shown_name(&self) -> String {
        if self.link.service().is_some() {
            self.link.name.clone()
        } else {
            self.settings.device_name.clone()
        }
    }

    pub(super) fn save_name(&mut self) {
        let name = self.name_edit.text().trim().to_owned();
        let Some(config) = &self.config else {
            return;
        };
        // 监听线程一秒内读到配置变化，同步服务按新名字重开
        if let Err(error) = ClipSettings::write_device_name(config, &name) {
            tracing::warn!(%error, "cannot write device name");
        }
    }

    pub(super) fn submit_pairing(&mut self) {
        let Pairing::Entering { address, code } = &self.pairing else {
            return;
        };
        let Some(service) = self.link.service() else {
            return;
        };
        let Ok(target) = address.text().trim().parse::<SocketAddr>() else {
            self.pair_status = crate::i18n::tr("devices.bad_address").to_owned();
            return;
        };
        self.pair_status.clear();
        service.join_pairing(target, code.text().trim().to_owned());
    }

    pub(super) fn cancel_pairing(&mut self) {
        if let Some(service) = self.link.service() {
            service.cancel_pairing();
        }
        self.pairing = Pairing::Idle;
    }

    pub(super) fn close_devices(&mut self) {
        self.cancel_pairing();
        if let Some(service) = self.link.service() {
            service.set_watching(false);
        }
        self.showing_devices = false;
        self.reload();
    }
}
