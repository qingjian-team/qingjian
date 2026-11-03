//! 接上窗口框架：建树、处理消息、字体，以及窗口的显隐。

use std::sync::Arc;
use std::sync::mpsc;

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{
    App, Edit, FamilyList, FamilyTable, Key, KeyCode, RenderError, TextEdit, TextPainter, Ui,
    Waker, WindowCommand, set_clipboard, system_fonts,
};

use super::ClipboardApp;
use super::menu::OpenedMenu;
use super::message::Message;
use crate::link::Pairing;
use crate::system;
use crate::view;

/// 代码用的等宽字体，按顺序挑系统里有的。
const MONO_FONTS: [&str; 3] = ["Cascadia Mono", "Consolas", "Courier New"];

/// 界面字体：Windows 11 的；没有时回到引擎的 Segoe UI。
const UI_FONTS: [&str; 2] = ["Segoe UI Variable", FamilyList::SYSTEM];

/// 上面那个字体在 DirectWrite 里的字族名，找文件要用它；加载后字体库里叫 `Segoe UI Variable`。
const UI_FONT_FILES: [&str; 1] = ["Segoe UI Variable Text"];

/// 图标字体：Windows 11 的，Windows 10 的。
const ICON_FONTS: [&str; 2] = ["Segoe Fluent Icons", "Segoe MDL2 Assets"];

impl App for ClipboardApp {
    type Message = Message;

    fn view(
        &self,
        ui: &mut Ui<Message>,
        scale: f32,
        size: (u32, u32),
    ) -> Result<NodeId, RenderError> {
        view::view(ui, self, scale, size)
    }

    fn update(&mut self, message: Message) -> bool {
        match message {
            Message::Search(_) if self.hotkey_still_held() => {}
            // 卡片横着排、搜索框又是空的：左右键没有光标可挪，用来换卡片
            Message::Search(Edit::Key(press))
                if self.edge.horizontal()
                    && self.search.text().is_empty()
                    && matches!(press.key, KeyCode::ArrowLeft | KeyCode::ArrowRight) =>
            {
                self.move_selection(press.key == KeyCode::ArrowRight);
            }
            Message::Search(edit) => {
                let before = self.search.text().to_owned();
                if let Some(copied) = self.search.apply(&edit) {
                    set_clipboard(&copied);
                }
                if self.search.text() != before {
                    self.reload();
                }
            }
            Message::Move(_) | Message::PasteSelected if self.showing_devices => {}
            Message::Close if self.showing_devices => self.close_devices(),
            Message::OpenDevices => {
                self.showing_devices = true;
                if let Some(service) = self.link.service() {
                    service.set_watching(true);
                }
                self.pair_status.clear();
                self.name_edit = TextEdit::new(self.shown_name());
            }
            Message::CloseDevices => self.close_devices(),
            Message::NameChange(edit) => {
                if let Some(copied) = self.name_edit.apply(&edit) {
                    set_clipboard(&copied);
                }
            }
            Message::NameSave => self.save_name(),
            Message::PairShowCode => {
                let code = self.link.service().map(|service| service.begin_pairing());
                match code {
                    Some(Ok(code)) => {
                        self.pair_status.clear();
                        self.pairing = Pairing::Showing(code);
                    }
                    Some(Err(error)) => tracing::warn!(%error, "cannot make a pairing code"),
                    None => {}
                }
            }
            Message::PairEnter(address) => {
                self.pair_status.clear();
                self.pairing = Pairing::Entering {
                    address: TextEdit::new(address.as_str()),
                    code: TextEdit::new(""),
                };
            }
            Message::PairAddress(edit) => {
                if let Pairing::Entering { address, .. } = &mut self.pairing
                    && let Some(copied) = address.apply(&edit)
                {
                    set_clipboard(&copied);
                }
            }
            Message::PairCode(edit) => {
                if let Pairing::Entering { code, .. } = &mut self.pairing
                    && let Some(copied) = code.apply(&edit)
                {
                    set_clipboard(&copied);
                }
            }
            Message::PairSubmit => self.submit_pairing(),
            Message::PairCancel => self.cancel_pairing(),
            Message::Unpair(device_id) => {
                if let Some(service) = self.link.service()
                    && let Err(error) = service.unpair(&device_id)
                {
                    tracing::warn!(%error, "cannot unpair");
                }
            }
            Message::Move(down) => self.move_selection(down),
            // 菜单开着时回车、Esc 只管菜单
            Message::PasteSelected | Message::Close if self.menu.is_some() => self.menu = None,
            Message::PasteSelected => self.paste(self.selected),
            Message::Paste(index) => self.paste(index),
            Message::Copy(index) => self.copy(index),
            Message::Filter(filter) => {
                self.filter = filter;
                self.list_generation += 1;
                self.reload();
            }
            Message::OpenMenu(index, position) => {
                self.selected = index;
                self.menu = Some(OpenedMenu { index, position });
            }
            Message::MenuPick(pick) => self.pick_menu(pick),
            Message::CloseMenu => self.menu = None,
            Message::EditChange(edit) => {
                if let Some(editing) = &mut self.editing
                    && let Some(copied) = editing.text.apply(&edit)
                {
                    set_clipboard(&copied);
                }
            }
            Message::EditSave => self.save_edit(),
            Message::EditCancel => self.editing = None,
            Message::Close => self.dismiss(),
        }
        true
    }

    fn setup(&mut self, text: &mut TextPainter) {
        let files: Vec<_> = MONO_FONTS
            .iter()
            .chain(&ICON_FONTS)
            .chain(&UI_FONT_FILES)
            .flat_map(|name| system_fonts::family_files(name))
            .collect();
        text.load_fonts(&files);
        let mut table = FamilyTable::new(FamilyList::system());
        let chain = MONO_FONTS.iter().map(|name| (*name).to_owned()).collect();
        self.mono = table.intern(&FamilyList(chain));
        let chain = ICON_FONTS.iter().map(|name| (*name).to_owned()).collect();
        self.icon_font = table.intern(&FamilyList(chain));
        let chain = UI_FONTS.iter().map(|name| (*name).to_owned()).collect();
        self.ui_font = table.intern(&FamilyList(chain));
        text.use_families(&Arc::new(table));
    }

    fn set_waker(&mut self, waker: Waker) {
        let (sender, receiver) = mpsc::channel();
        self.signals = Some(receiver);
        self.link.set_outbox(sender.clone(), waker.clone());
        self.link.apply(&self.settings);
        if let Some(watch) = self.watch.take() {
            system::spawn_listener(watch, sender, waker);
        }
    }

    fn wake(&mut self) -> bool {
        let Some(signals) = self.signals.take() else {
            return false;
        };
        let mut changed = false;
        while let Ok(signal) = signals.try_recv() {
            self.handle(signal);
            changed = true;
        }
        self.signals = Some(signals);
        changed
    }

    fn spotlight(&self) -> Option<Key> {
        let item = self.items.get(self.selected)?;
        Some(Key::ROOT.child(view::row_key(item)))
    }

    /// 点到别处就收起。
    fn set_active(&mut self, active: bool) {
        if !active && self.visible {
            self.hide();
        }
    }

    fn take_window_command(&mut self) -> Option<WindowCommand> {
        self.command.take()
    }

    fn set_system_dark(&mut self, dark: bool) {
        self.dark = self.forced_dark.unwrap_or(dark);
    }
}
