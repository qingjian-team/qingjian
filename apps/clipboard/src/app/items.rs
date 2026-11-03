//! 对列表里一条的操作：粘贴、复制、编辑、删除。

use bamboo_slip::TextEdit;

use super::editing::Editing;
use super::menu::{MENU_COPY, MENU_DELETE, MENU_EDIT, MENU_PASTE};
use super::{ClipboardApp, now_millis};
use crate::system::{self, Payload};

impl ClipboardApp {
    /// 这一条要写回系统剪贴板的内容，并把它挪到最前。
    fn take_payload(&mut self, index: usize) -> Option<Payload> {
        let item = self.items.get(index)?;
        let formats = self
            .store
            .formats(&item.id)
            .inspect_err(|error| tracing::warn!(%error, "clipboard formats read failed"))
            .ok()?;
        let payload = Payload {
            kind: item.kind,
            text: item.text.clone(),
            formats,
        };
        if let Err(error) = self.store.touch(&item.id, now_millis()) {
            tracing::warn!(%error, "clipboard touch failed");
        }
        self.link.changed();
        Some(payload)
    }

    pub(super) fn paste(&mut self, index: usize) {
        if let Some(payload) = self.take_payload(index) {
            system::paste(payload, self.target);
            self.hide();
        }
    }

    pub(super) fn copy(&mut self, index: usize) {
        if let Some(payload) = self.take_payload(index) {
            system::copy(payload, self.target);
            self.hide();
        }
    }

    pub(super) fn pick_menu(&mut self, pick: usize) {
        let Some(menu) = self.menu.take() else {
            return;
        };
        let Some(item) = self.items.get(menu.index) else {
            return;
        };
        match pick {
            MENU_PASTE => self.paste(menu.index),
            MENU_COPY => self.copy(menu.index),
            MENU_EDIT if item.kind.editable() => {
                self.editing = Some(Editing {
                    id: item.id.clone(),
                    text: TextEdit::multiline(item.text.clone()),
                });
            }
            MENU_DELETE => {
                let id = item.id.clone();
                if let Err(error) = self.store.delete(&id, now_millis()) {
                    tracing::warn!(%error, "clipboard delete failed");
                }
                self.link.changed();
                self.thumbnails.remove(&id);
                let selected = self.selected;
                self.reload();
                self.selected = selected.min(self.items.len().saturating_sub(1));
            }
            _ => {}
        }
    }

    pub(super) fn save_edit(&mut self) {
        let Some(editing) = self.editing.take() else {
            return;
        };
        if let Err(error) = self
            .store
            .edit_text(&editing.id, editing.text.text(), now_millis())
        {
            tracing::warn!(%error, "clipboard edit failed");
        }
        self.link.changed();
        self.reload();
    }
}
