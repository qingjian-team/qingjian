//! 面板和同步服务之间：按配置开关服务，把服务的事件转成面板的 [`Signal`]，配对的状态也放这里。

mod pairing;

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, channel};

use bamboo_slip::Waker;
use qingjian_clipboard::{ClipSettings, StoreReplica};
use qingjian_clipboard_sync::default_name::default_device_name;
use qingjian_clipboard_sync::{Identity, SyncOptions, SyncService};

use crate::picture;
use crate::system::{Signal, computer_name};

pub use pairing::Pairing;

pub struct SyncLink {
    service: Option<SyncService>,

    /// 数据库文件，同步数据放它旁边。
    database: PathBuf,

    outbox: Option<(Sender<Signal>, Waker)>,

    /// 服务现在用的名字。
    pub name: String,
}

impl SyncLink {
    pub fn new(database: &Path) -> Self {
        Self {
            service: None,
            database: database.to_owned(),
            outbox: None,
            name: String::new(),
        }
    }

    pub fn set_outbox(&mut self, sender: Sender<Signal>, waker: Waker) {
        self.outbox = Some((sender, waker));
    }

    pub fn service(&self) -> Option<&SyncService> {
        self.service.as_ref()
    }

    fn directory(&self) -> PathBuf {
        self.database
            .parent()
            .map_or_else(|| PathBuf::from("sync"), |parent| parent.join("sync"))
    }

    /// 配置说开就开，说关就关。名字或条数上限变了要重开。
    pub fn apply(&mut self, settings: &ClipSettings) {
        self.service = None;
        if !settings.sync_enabled {
            return;
        }
        match self.start(settings) {
            Ok(service) => self.service = Some(service),
            Err(error) => tracing::warn!(%error, "cannot start clipboard sync"),
        }
    }

    fn start(
        &mut self,
        settings: &ClipSettings,
    ) -> Result<SyncService, Box<dyn std::error::Error>> {
        let directory = self.directory();
        let identity = Identity::load_or_create(&directory.join("identity.key"))?;
        let chinese = settings.locale().starts_with("zh") || settings.locale().is_empty();
        self.name = match settings.device_name.trim() {
            "" => default_device_name(&identity.public, chinese),
            name => name.to_owned(),
        };
        let replica =
            StoreReplica::open(&self.database, settings.history_limit, picture::from_file)?;
        let (sender, events) = channel();
        if let Some((outbox, waker)) = self.outbox.clone() {
            std::thread::Builder::new()
                .name("clipboard-sync-events".to_owned())
                .spawn(move || {
                    while let Ok(event) = events.recv() {
                        if outbox.send(Signal::Sync(event)).is_err() {
                            return;
                        }
                        waker.wake();
                    }
                })?;
        }
        Ok(SyncService::start(
            SyncOptions {
                directory,
                name: self.name.clone(),
                model: computer_name(),
                own_discovery: true,
            },
            Box::new(replica),
            sender,
        )?)
    }

    /// 本机内容变了。
    pub fn changed(&self) {
        if let Some(service) = &self.service {
            service.notify_changed();
        }
    }
}
