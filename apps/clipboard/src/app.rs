//! 面板的状态与消息：后台线程送来的复制与快捷键在 `wake` 里处理，选中一条就藏起面板、粘贴回原窗口。

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::time::{SystemTime, UNIX_EPOCH};

use bamboo_slip::taffy::NodeId;
use bamboo_slip::tiny_skia::Pixmap;
use bamboo_slip::{
    Accent, App, Edit, FamilyId, FamilyList, FamilyTable, Key, KeyCode, RenderError, TextEdit,
    TextPainter, Ui, Waker, WindowCommand, set_clipboard, system_fonts,
};
use qingjian_clipboard::{
    ClipFilter, ClipItem, ClipKind, ClipSettings, ClipStore, Edge, FORMAT_FILE_SIZE,
    FORMAT_THUMBNAIL, format_size,
};
use qingjian_clipboard_sync::{ReceivedItem, SyncEvent};

use crate::link::{Pairing, SyncLink};
use crate::system::{self, Neon, Payload, Signal};
use crate::view;

/// 面板最多列多少条，再多靠搜索。
const SHOWN_ITEMS: usize = 200;

/// 代码用的等宽字体，按顺序挑系统里有的。
const MONO_FONTS: [&str; 3] = ["Cascadia Mono", "Consolas", "Courier New"];

/// 界面字体：Windows 11 的；没有时回到引擎的 Segoe UI。
const UI_FONTS: [&str; 2] = ["Segoe UI Variable", FamilyList::SYSTEM];

/// 上面那个字体在 DirectWrite 里的字族名，找文件要用它；加载后字体库里叫 `Segoe UI Variable`。
const UI_FONT_FILES: [&str; 1] = ["Segoe UI Variable Text"];

/// 图标字体：Windows 11 的，Windows 10 的。
const ICON_FONTS: [&str; 2] = ["Segoe Fluent Icons", "Segoe MDL2 Assets"];

#[derive(Debug, Clone)]
pub enum Message {
    Search(Edit),

    /// 上下键移动选中项；`true` 往后。
    Move(bool),

    /// 粘贴选中项。
    PasteSelected,

    Paste(usize),

    /// 只放上剪贴板，不粘贴。
    Copy(usize),

    Filter(ClipFilter),

    /// 在这个位置（像素）给这一条开右键菜单。
    OpenMenu(usize, (f32, f32)),

    MenuPick(usize),

    CloseMenu,

    EditChange(Edit),

    EditSave,

    EditCancel,

    OpenDevices,

    CloseDevices,

    NameChange(Edit),

    NameSave,

    PairShowCode,

    /// 开始输入配对码，地址先填上（可能是空的）。
    PairEnter(String),

    PairAddress(Edit),

    PairCode(Edit),

    PairSubmit,

    PairCancel,

    Unpair(String),

    Close,
}

/// 右键菜单的几项，序号与 `view::menu_items` 对应。
pub const MENU_PASTE: usize = 0;

pub const MENU_COPY: usize = 1;

pub const MENU_EDIT: usize = 2;

pub const MENU_DELETE: usize = 3;

/// 开着的右键菜单。
pub struct OpenedMenu {
    pub index: usize,

    pub position: (f32, f32),
}

/// 正在编辑的一条。
pub struct Editing {
    pub id: String,

    pub text: TextEdit,
}

pub struct ClipboardApp {
    store: ClipStore,

    edge: Edge,

    /// 暂停记录中，面板上写一句，免得以为坏了。
    pub(crate) paused: bool,

    /// 命令行指定了方向：配置里的不算。
    fixed_edge: bool,

    /// 交给监听线程的配置，开窗后取走。
    watch: Option<system::Watch>,

    signals: Option<Receiver<Signal>>,

    pub(crate) search: TextEdit,

    pub(crate) filter: ClipFilter,

    pub(crate) items: Vec<ClipItem>,

    pub(crate) selected: usize,

    pub(crate) menu: Option<OpenedMenu>,

    pub(crate) editing: Option<Editing>,

    /// 卡片上的缩略图，按条目 id。
    pub(crate) thumbnails: HashMap<String, Arc<Pixmap>>,

    /// 文件的总大小、图片的尺寸与大小（已写成给人看的样子），按条目 id；不知道的没有。
    pub(crate) size_labels: HashMap<String, String>,

    /// 来源程序的图标，按程序路径；取不到的也记下，不反复取。
    pub(crate) icons: HashMap<String, Option<Arc<Pixmap>>>,

    pub(crate) mono: FamilyId,

    pub(crate) icon_font: FamilyId,

    pub(crate) ui_font: FamilyId,

    pub(crate) dark: bool,

    /// 命令行指定的外观；有它就不跟系统。
    forced_dark: Option<bool>,

    pub(crate) accent: Accent,

    /// 窗口开了亚克力材质。
    pub(crate) acrylic: bool,

    /// 滚动位置按滚动区的 key 记：换一下 key，下次打开或换筛选时就回到开头。
    pub(crate) list_generation: u64,

    visible: bool,

    /// 呼出时还按着的快捷键主键。面板比手快：按键重复会直接打进搜索框，松开之前都不算打字。
    held_key: Option<u32>,

    /// 呼出面板前的前台窗口，粘贴回它。
    target: isize,

    command: Option<WindowCommand>,

    pub(crate) link: SyncLink,

    neon: Neon,

    /// 同步时亮霓虹灯。
    neon_on: bool,

    /// 上次的配置，看同步相关的键变了没有。
    settings: ClipSettings,

    /// 配置文件，改设备名时写。
    config: Option<PathBuf>,

    pub(crate) showing_devices: bool,

    pub(crate) name_edit: TextEdit,

    pub(crate) pairing: Pairing,

    /// 配对的结果，显示在设备页。
    pub(crate) pair_status: String,
}

impl ClipboardApp {
    pub fn new(
        store: ClipStore,
        database: &Path,
        watch: system::Watch,
        forced_edge: Option<Edge>,
        forced_dark: Option<bool>,
        acrylic: bool,
    ) -> Self {
        let settings = watch.settings.clone();
        let config = watch.path.clone();
        Self {
            store,
            link: SyncLink::new(database),
            neon: Neon::spawn(),
            neon_on: settings.sync_neon,
            name_edit: TextEdit::new(settings.device_name.as_str()),
            settings,
            config,
            showing_devices: false,
            pairing: Pairing::Idle,
            pair_status: String::new(),
            paused: watch.settings.paused,
            edge: forced_edge.unwrap_or(watch.settings.edge),
            fixed_edge: forced_edge.is_some(),
            watch: Some(watch),
            signals: None,
            search: TextEdit::new(""),
            filter: ClipFilter::All,
            items: Vec::new(),
            selected: 0,
            menu: None,
            editing: None,
            thumbnails: HashMap::new(),
            size_labels: HashMap::new(),
            icons: HashMap::new(),
            mono: FamilyId::default(),
            icon_font: FamilyId::default(),
            ui_font: FamilyId::default(),
            dark: forced_dark.unwrap_or(false),
            forced_dark,
            accent: system_accent(),
            acrylic,
            list_generation: 0,
            visible: false,
            target: 0,
            held_key: None,
            command: None,
        }
    }

    fn reload(&mut self) {
        self.items = self
            .store
            .search(self.search.text(), self.filter, SHOWN_ITEMS)
            .inspect_err(|error| tracing::warn!(%error, "clipboard search failed"))
            .unwrap_or_default();
        self.selected = 0;
        self.menu = None;
        // 删掉、清空了的缩略图不留在内存里
        let items = &self.items;
        self.thumbnails
            .retain(|id, _| items.iter().any(|item| item.id == *id));
        self.load_assets();
    }

    /// 卡片要用的缩略图与图标放在内存里，建树时不碰磁盘。
    fn load_assets(&mut self) {
        for item in &self.items {
            if item.kind == ClipKind::Image && !self.thumbnails.contains_key(&item.id) {
                let thumbnail = self
                    .store
                    .format(&item.id, FORMAT_THUMBNAIL)
                    .ok()
                    .flatten()
                    .and_then(|png| Pixmap::decode_png(&png).ok());
                if let Some(thumbnail) = thumbnail {
                    self.thumbnails.insert(item.id.clone(), Arc::new(thumbnail));
                }
            }
            if item.kind == ClipKind::Image && !self.size_labels.contains_key(&item.id) {
                let info = self.store.image_info(&item.id).ok().flatten();
                if let Some((width, height, size)) = info {
                    self.size_labels.insert(
                        item.id.clone(),
                        format!("{width} × {height} · {}", format_size(size)),
                    );
                }
            }
            if item.kind == ClipKind::Files && !self.size_labels.contains_key(&item.id) {
                let size = self
                    .store
                    .format(&item.id, FORMAT_FILE_SIZE)
                    .ok()
                    .flatten()
                    .and_then(|text| String::from_utf8(text).ok())
                    .and_then(|text| text.parse::<u64>().ok());
                if let Some(size) = size {
                    self.size_labels.insert(item.id.clone(), format_size(size));
                }
            }
            if !item.source.is_empty() && !self.icons.contains_key(&item.source) {
                let icon = system::load_icon(&item.source).map(Arc::new);
                self.icons.insert(item.source.clone(), icon);
            }
        }
    }

    fn move_selection(&mut self, forward: bool) {
        let last = self.items.len().saturating_sub(1);
        self.selected = if forward {
            (self.selected + 1).min(last)
        } else {
            self.selected.saturating_sub(1)
        };
    }

    /// 呼出用的主键还没松开；松开过一次就不再看。
    fn hotkey_still_held(&mut self) -> bool {
        let held = self.held_key.is_some_and(system::key_down);
        if !held {
            self.held_key = None;
        }
        held
    }

    pub(crate) fn horizontal(&self) -> bool {
        self.edge.horizontal()
    }

    fn hide(&mut self) {
        self.visible = false;
        self.menu = None;
        self.editing = None;
        // 下次呼出回到列表；显示着的配对码也随之作废
        if self.showing_devices {
            self.close_devices();
        }
        self.command = Some(WindowCommand::Hide);
    }

    /// 主动收起：焦点还给原窗口。点到别处收起时不走这里，焦点已经在用户点的窗口上。
    fn dismiss(&mut self) {
        self.hide();
        system::refocus(self.target);
    }

    fn handle(&mut self, signal: Signal) {
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

    fn shown_name(&self) -> String {
        if self.link.service().is_some() {
            self.link.name.clone()
        } else {
            self.settings.device_name.clone()
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

    fn save_name(&mut self) {
        let name = self.name_edit.text().trim().to_owned();
        let Some(config) = &self.config else {
            return;
        };
        // 监听线程一秒内读到配置变化，同步服务按新名字重开
        if let Err(error) = ClipSettings::write_device_name(config, &name) {
            tracing::warn!(%error, "cannot write device name");
        }
    }

    fn submit_pairing(&mut self) {
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

    fn paste(&mut self, index: usize) {
        if let Some(payload) = self.take_payload(index) {
            system::paste(payload, self.target);
            self.hide();
        }
    }

    fn copy(&mut self, index: usize) {
        if let Some(payload) = self.take_payload(index) {
            system::copy(payload, self.target);
            self.hide();
        }
    }

    fn pick_menu(&mut self, pick: usize) {
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

    fn cancel_pairing(&mut self) {
        if let Some(service) = self.link.service() {
            service.cancel_pairing();
        }
        self.pairing = Pairing::Idle;
    }

    fn close_devices(&mut self) {
        self.cancel_pairing();
        if let Some(service) = self.link.service() {
            service.set_watching(false);
        }
        self.showing_devices = false;
        self.reload();
    }

    fn save_edit(&mut self) {
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

fn system_accent() -> Accent {
    Accent::system().unwrap_or(Accent::FLUENT_BLUE)
}

pub(crate) fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as i64)
}
