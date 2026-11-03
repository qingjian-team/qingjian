//! 面板的状态与消息：后台线程送来的复制与快捷键在 `wake` 里处理，选中一条就藏起面板、粘贴回原窗口。

mod devices;
mod editing;
mod items;
mod menu;
mod message;
mod signal;
mod window;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::time::{SystemTime, UNIX_EPOCH};

use bamboo_slip::tiny_skia::Pixmap;
use bamboo_slip::{Accent, FamilyId, TextEdit, WindowCommand};
use qingjian_clipboard::{
    ClipFilter, ClipItem, ClipKind, ClipSettings, ClipStore, Edge, FORMAT_FILE_SIZE,
    FORMAT_THUMBNAIL, format_size,
};

use crate::link::{Pairing, SyncLink};
use crate::system::{self, Neon, Signal};

pub use editing::Editing;
pub use menu::{MENU_EDIT, OpenedMenu};
pub use message::Message;

/// 面板最多列多少条，再多靠搜索。
const SHOWN_ITEMS: usize = 200;

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
}

fn system_accent() -> Accent {
    Accent::system().unwrap_or(Accent::FLUENT_BLUE)
}

pub(crate) fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as i64)
}
