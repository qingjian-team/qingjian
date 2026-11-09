//! 三个线程共用的状态。

mod nearby;
mod pending_code;

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::time::{SystemTime, UNIX_EPOCH};

use super::event::SyncEvent;
use super::link::Link;
use super::lock;
use super::locked::LockedReplica;
use crate::identity::Identity;
use crate::peers::PeerBook;

pub use nearby::Nearby;
pub(super) use pending_code::PendingCode;

pub(super) struct Shared {
    pub identity: Identity,

    pub name: String,

    /// 型号或电脑名。
    pub model: String,

    /// 监听端口，配对时报给对方。
    pub port: u16,

    pub book: Mutex<PeerBook>,

    pub replica: LockedReplica,

    pub nearby: Mutex<HashMap<String, Nearby>>,

    pub pending: Mutex<Option<PendingCode>>,

    /// 各已配对设备的连接结果，按设备 id。
    pub links: Mutex<HashMap<String, Link>>,

    /// 设备页开着：对账勤一些，状态才是新的。
    pub watching: AtomicBool,

    pub scanning: AtomicBool,

    /// 上次扫描找到的设备，下次扫不到就从附近设备里拿掉。
    pub scanned: Mutex<HashSet<String>>,

    events: Sender<SyncEvent>,
}

impl Shared {
    pub fn new(
        identity: Identity,
        name: String,
        model: String,
        port: u16,
        book: PeerBook,
        replica: LockedReplica,
        events: Sender<SyncEvent>,
    ) -> Self {
        Self {
            identity,
            name,
            model,
            port,
            book: Mutex::new(book),
            replica,
            nearby: Mutex::new(HashMap::new()),
            pending: Mutex::new(None),
            links: Mutex::new(HashMap::new()),
            watching: AtomicBool::new(false),
            scanning: AtomicBool::new(false),
            scanned: Mutex::new(HashSet::new()),
            events,
        }
    }

    /// 记一次连接结果；状态变了告诉界面。
    pub fn record_link(&self, device_id: &str, ok: bool) {
        let now = now_millis();
        let (before, after) = {
            let mut links = lock(&self.links);
            let link = links.entry(device_id.to_owned()).or_default();
            let before = link.status(now);
            if ok {
                link.last_ok = Some(now);
            } else {
                link.last_failure = Some(now);
            }
            (before, link.status(now))
        };
        if ok && let Err(error) = lock(&self.book).touch(device_id, now, None) {
            tracing::warn!(%error, "cannot save last seen");
        }
        if before != after {
            self.emit(SyncEvent::NearbyChanged);
        }
    }

    /// 界面退出后事件没人收，丢掉就好。
    pub fn emit(&self, event: SyncEvent) {
        let _ = self.events.send(event);
    }
}

pub(super) fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as i64)
}
