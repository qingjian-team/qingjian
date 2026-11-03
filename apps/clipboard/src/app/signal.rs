//! 后台线程送来的事：复制、配置变化、呼出快捷键，以及同步服务的事件。

use bamboo_slip::{TextEdit, WindowCommand};
use qingjian_clipboard::ClipFilter;
use qingjian_clipboard_sync::{ReceivedItem, SyncEvent};

use super::{ClipboardApp, now_millis, system_accent};
use crate::link::Pairing;
use crate::system::{self, Payload, Signal};

impl ClipboardApp {
    pub(super) fn handle(&mut self, signal: Signal) {
        match signal {
            Signal::Copied(capture) => {
                if let Err(error) = self.store.add(&capture, now_millis()) {
                    tracing::warn!(%error, "clipboard record failed");
                }
                self.link.changed();
                if self.visible && self.editing.is_none() {
                    self.reload();
                }
            }
            Signal::Sync(event) => self.handle_sync(event),
            Signal::Settings(settings) => {
                let sync_changed = settings.sync_enabled != self.settings.sync_enabled
                    || settings.device_name != self.settings.device_name
                    || settings.history_limit != self.settings.history_limit
                    || settings.language != self.settings.language;
                self.neon_on = settings.sync_neon;
                self.settings = settings.clone();
                if sync_changed {
                    self.link.apply(&settings);
                    self.name_edit = TextEdit::new(self.shown_name());
                    if self.link.service().is_none() {
                        self.showing_devices = false;
                    }
                }
                self.paused = settings.paused;
                if let Err(error) = self.store.set_limit(settings.history_limit, now_millis()) {
                    tracing::warn!(%error, "clipboard limit change failed");
                }
                bamboo_slip_i18n::set_locale(settings.locale());
                if !self.fixed_edge && settings.edge != self.edge {
                    if self.visible {
                        self.dismiss();
                    }
                    self.edge = settings.edge;
                }
            }
            Signal::Hotkey {
                target,
                area,
                scale,
                held_key,
            } => {
                if self.visible {
                    self.dismiss();
                    return;
                }
                self.target = target;
                self.held_key = held_key;
                self.accent = system_accent();
                self.search = TextEdit::new("");
                self.filter = ClipFilter::All;
                self.list_generation += 1;
                self.reload();
                let (x, y, width, height) = self.edge.rect(area, scale);
                self.visible = true;
                self.command = Some(WindowCommand::Show {
                    x,
                    y,
                    width,
                    height,
                    from: Some(self.edge.slide_start(area, scale)),
                });
            }
        }
    }

    fn handle_sync(&mut self, event: SyncEvent) {
        match event {
            SyncEvent::NearbyChanged => {}
            SyncEvent::Paired { name } => {
                self.pairing = Pairing::Idle;
                self.pair_status = crate::i18n::tr("devices.paired_ok").replace("{name}", &name);
            }
            SyncEvent::PairFailed => {
                self.pairing = Pairing::Idle;
                self.pair_status = crate::i18n::tr("devices.pair_failed").to_owned();
            }
            SyncEvent::ReceiveStarted { .. } => {
                if self.neon_on {
                    self.neon.start();
                }
            }
            SyncEvent::ReceiveProgress {
                bytes_per_second, ..
            } => {
                if self.neon_on {
                    self.neon.speed(bytes_per_second);
                }
            }
            SyncEvent::ReceiveFinished { newest, .. } => {
                if self.neon_on {
                    self.neon.finish(true);
                }
                if let Some(newest) = newest {
                    self.put_on_clipboard(&newest);
                }
                if self.visible && self.editing.is_none() {
                    self.reload();
                }
            }
            SyncEvent::ReceiveFailed { .. } | SyncEvent::SendFailed { .. } => {
                if self.neon_on {
                    self.neon.finish(false);
                }
            }
        }
    }

    /// 收到的最新内容放上系统剪贴板，这边直接就能粘贴。监听线程认得这是自己写的，不会再记一遍、再传回去。
    fn put_on_clipboard(&self, received: &ReceivedItem) {
        let item = match self.store.item_by_sync_id(&received.id) {
            Ok(Some(item)) => item,
            Ok(None) => return,
            Err(error) => {
                tracing::warn!(%error, "received item read failed");
                return;
            }
        };
        let formats = self.store.formats(&item.id).unwrap_or_default();
        let payload = Payload {
            kind: item.kind,
            text: item.text,
            formats,
        };
        std::thread::spawn(move || system::write_payload(&payload));
    }
}
