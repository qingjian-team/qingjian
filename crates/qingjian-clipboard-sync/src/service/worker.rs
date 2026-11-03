//! 后台的几个线程：接受连接、mDNS 发现，以及按需或定时主动同步。

use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::command::Command;
use super::event::SyncEvent;
use super::link::PeerStatus;
use super::lock;
use super::round::{handle_incoming, sync_peer};
use super::scan;
use super::state::{Nearby, Shared, now_millis};
use crate::discovery::{Discovery, Found};

/// 隔多久主动对一次账，兜住错过的通知。
const RESYNC_INTERVAL: Duration = Duration::from_secs(120);

/// 设备页开着时的对账间隔，状态要跟得上。
const WATCH_INTERVAL: Duration = Duration::from_secs(10);

/// 有设备连不上时，隔多久扫一次找它的新地址。
const RECOVER_SCAN_INTERVAL: Duration = Duration::from_secs(300);

/// 设备页开着时隔多久重扫。
const WATCH_SCAN_INTERVAL: Duration = Duration::from_secs(30);

const POLL: Duration = Duration::from_millis(250);

pub(super) fn spawn_accept(
    listener: TcpListener,
    shared: Arc<Shared>,
    stop: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((stream, address)) => {
                    let shared = shared.clone();
                    thread::spawn(move || handle_incoming(stream, address, &shared));
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(POLL);
                }
                Err(error) => {
                    tracing::warn!(%error, "accept failed");
                    thread::sleep(POLL);
                }
            }
        }
    })
}

pub(super) fn spawn_discovery(
    mut discovery: Discovery,
    shared: Arc<Shared>,
    commands: Sender<Command>,
    stop: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            let Some(found) = discovery.next(POLL) else {
                continue;
            };
            apply_found(&shared, &commands, found);
        }
    })
}

pub(super) fn apply_found(shared: &Shared, commands: &Sender<Command>, found: Found) {
    match found {
        Found::Seen {
            device_id,
            name,
            model,
            address,
        } => {
            let fresh = lock(&shared.nearby)
                .insert(
                    device_id.clone(),
                    Nearby {
                        device_id: device_id.clone(),
                        name,
                        model,
                        address,
                    },
                )
                .is_none();
            // 发现了已配对设备的新地址就记下来，下次发现不了也连得上
            if let Err(error) =
                lock(&shared.book).touch(&device_id, now_millis(), Some(address.to_string()))
            {
                tracing::warn!(%error, "cannot save peer address");
            }
            if fresh {
                let _ = commands.send(Command::SyncPeer(device_id));
                shared.emit(SyncEvent::NearbyChanged);
            }
        }
        Found::Gone { device_id } => {
            lock(&shared.nearby).remove(&device_id);
            shared.emit(SyncEvent::NearbyChanged);
        }
    }
}

pub(super) fn spawn_worker(
    inbox: Receiver<Command>,
    shared: Arc<Shared>,
    commands: Sender<Command>,
    stop: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut last_all = Instant::now();
        let mut last_scan = Instant::now();
        while !stop.load(Ordering::Relaxed) {
            let command = match inbox.recv_timeout(POLL) {
                Ok(command) => Some(command),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => return,
            };
            // 连着来的「全部同步」并成一轮
            if matches!(command, Some(Command::SyncAll)) {
                while matches!(inbox.try_recv(), Ok(Command::SyncAll)) {}
            }
            let only = match command {
                Some(Command::SyncPeer(id)) => Some(id),
                Some(Command::SyncAll) => None,
                None if last_all.elapsed() >= resync_interval(&shared) => None,
                None => continue,
            };
            if only.is_none() {
                last_all = Instant::now();
            }
            // 先取出来：写在 for 头上锁会拿到循环结束，两台设备互连时都等对方的名单就死锁了
            let peers = lock(&shared.book).peers().to_vec();
            for peer in peers {
                if only.as_ref().is_none_or(|id| *id == peer.device_id) {
                    sync_peer(&shared, &peer);
                }
            }
            if last_scan.elapsed() >= scan_interval(&shared) {
                last_scan = Instant::now();
                scan::start(shared.clone(), commands.clone());
            }
        }
    })
}

fn resync_interval(shared: &Shared) -> Duration {
    if shared.watching.load(Ordering::Relaxed) {
        WATCH_INTERVAL
    } else {
        RESYNC_INTERVAL
    }
}

/// 没人看、也没有连不上的设备时不扫：一遍要连两百多个地址。
fn scan_interval(shared: &Shared) -> Duration {
    let now = now_millis();
    let peers = lock(&shared.book).peers().to_vec();
    let links = lock(&shared.links);
    let lost = peers.iter().any(|peer| {
        links
            .get(&peer.device_id)
            .is_some_and(|link| link.status(now) == PeerStatus::Unreachable)
    });
    if shared.watching.load(Ordering::Relaxed) {
        WATCH_SCAN_INTERVAL
    } else if lost {
        RECOVER_SCAN_INTERVAL
    } else {
        Duration::MAX
    }
}
