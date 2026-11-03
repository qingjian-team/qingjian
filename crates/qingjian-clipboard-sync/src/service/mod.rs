//! [`SyncService`]：后台跑同步。打开同步时才建它，它一建就监听端口并公布自己；丢掉它就全停。
//! 线程三个：接受连接、mDNS 发现、主动同步。平台只提供 [`Replica`]，并把事件接到界面。

#[cfg(test)]
mod tests;

mod event;
mod hello;
mod link;
mod locked;
mod round;
mod scan;
mod state;

pub use event::{ReceivedItem, SyncEvent};
pub use link::{PeerState, PeerStatus};
pub use state::Nearby;

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::channel::{generate_pair_code, pair_join};
use crate::discovery::{Discovery, Found};
use crate::error::SyncError;
use crate::identity::Identity;
use crate::peers::{Peer, PeerBook, hex};
use crate::sync::Replica;
use hello::PairHello;
use locked::LockedReplica;
use round::{handle_incoming, sync_peer};
use state::{PendingCode, Shared, now_millis};

/// 监听端口的首选，被占了由系统分配，反正靠 mDNS 公布。
const PREFERRED_PORT: u16 = 53319;

/// 配对码多久过期。
const PAIR_CODE_LIFETIME: Duration = Duration::from_secs(120);

const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

/// 读写超时：连上之后对方不说话多久算断。
pub(crate) const IO_TIMEOUT: Duration = Duration::from_secs(15);

/// 隔多久主动对一次账，兜住错过的通知。
const RESYNC_INTERVAL: Duration = Duration::from_secs(120);

/// 设备页开着时的对账间隔，状态要跟得上。
const WATCH_INTERVAL: Duration = Duration::from_secs(10);

/// 有设备连不上时，隔多久扫一次找它的新地址。
const RECOVER_SCAN_INTERVAL: Duration = Duration::from_secs(300);

/// 设备页开着时隔多久重扫。
const WATCH_SCAN_INTERVAL: Duration = Duration::from_secs(30);

const POLL: Duration = Duration::from_millis(250);

pub struct SyncOptions {
    /// 身份和配对名单放的目录。
    pub directory: PathBuf,

    /// 给别的设备看的名字。
    pub name: String,

    /// 型号或电脑名，显示在名字旁边。
    pub model: String,

    /// 自己用 mDNS 公布和发现。安卓关掉，由系统的 NsdManager 代劳，结果经 [`SyncService::report`] 交进来。
    pub own_discovery: bool,
}

enum Command {
    SyncAll,
    SyncPeer(String),
}

pub struct SyncService {
    shared: Arc<Shared>,

    commands: Sender<Command>,

    stop: Arc<AtomicBool>,

    threads: Vec<JoinHandle<()>>,

    port: u16,
}

impl SyncService {
    pub fn start(
        options: SyncOptions,
        replica: Box<dyn Replica + Send>,
        events: Sender<SyncEvent>,
    ) -> Result<Self, SyncError> {
        let identity = Identity::load_or_create(&options.directory.join("identity.key"))?;
        let book = PeerBook::open(&options.directory.join("peers.json"))?;
        let listener = TcpListener::bind(("0.0.0.0", PREFERRED_PORT))
            .or_else(|_| TcpListener::bind(("0.0.0.0", 0)))?;
        let port = listener.local_addr()?.port();
        listener.set_nonblocking(true)?;

        let discovery = if options.own_discovery {
            Some(Discovery::start(
                &identity.device_id(),
                &options.name,
                &options.model,
                port,
            )?)
        } else {
            None
        };
        let shared = Arc::new(Shared::new(
            identity,
            options.name,
            options.model,
            port,
            book,
            LockedReplica::new(replica),
            events,
        ));
        let stop = Arc::new(AtomicBool::new(false));
        let (commands, inbox) = channel();
        // 一启动就对一轮：状态不用等，离线时攒下的也马上补
        let _ = commands.send(Command::SyncAll);
        let mut threads = vec![
            spawn_accept(listener, shared.clone(), stop.clone()),
            spawn_worker(inbox, shared.clone(), commands.clone(), stop.clone()),
        ];
        if let Some(discovery) = discovery {
            threads.push(spawn_discovery(
                discovery,
                shared.clone(),
                commands.clone(),
                stop.clone(),
            ));
        }
        Ok(Self {
            shared,
            commands,
            stop,
            threads,
            port,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// 交进来一条发现的结果（平台自己做发现时用）。
    pub fn report(&self, found: Found) {
        apply_found(&self.shared, &self.commands, found);
    }

    /// 本机内容变了（复制、删除、清空）：马上推给在线的设备。
    pub fn notify_changed(&self) {
        let _ = self.commands.send(Command::SyncAll);
    }

    /// 这台显示配对码：生成一个新码，旧码作废。
    pub fn begin_pairing(&self) -> Result<String, SyncError> {
        let code = generate_pair_code()?;
        *lock(&self.shared.pending) = Some(PendingCode {
            code: code.clone(),
            expires: Instant::now() + PAIR_CODE_LIFETIME,
        });
        Ok(code)
    }

    pub fn cancel_pairing(&self) {
        *lock(&self.shared.pending) = None;
    }

    /// 这台输入配对码。在后台连，结果走事件。
    pub fn join_pairing(&self, address: SocketAddr, code: String) {
        let shared = self.shared.clone();
        let commands = self.commands.clone();
        thread::spawn(move || {
            let event = match join(&shared, address, &code) {
                Ok(name) => {
                    let _ = commands.send(Command::SyncAll);
                    SyncEvent::Paired { name }
                }
                Err(error) => {
                    tracing::info!(%error, "pairing failed");
                    SyncEvent::PairFailed
                }
            };
            shared.emit(event);
        });
    }

    pub fn nearby(&self) -> Vec<Nearby> {
        lock(&self.shared.nearby).values().cloned().collect()
    }

    pub fn peers(&self) -> Vec<Peer> {
        lock(&self.shared.book).peers().to_vec()
    }

    /// 已配对设备和它们现在的状态。
    pub fn peer_states(&self) -> Vec<PeerState> {
        let now = now_millis();
        let links = lock(&self.shared.links).clone();
        self.peers()
            .into_iter()
            .map(|peer| PeerState {
                status: links
                    .get(&peer.device_id)
                    .copied()
                    .unwrap_or_default()
                    .status(now),
                peer,
            })
            .collect()
    }

    /// 设备页开了或关了。开着时马上探一次、扫一遍，之后勤一些。
    pub fn set_watching(&self, watching: bool) {
        self.shared.watching.store(watching, Ordering::Relaxed);
        if watching {
            let _ = self.commands.send(Command::SyncAll);
            scan::start(self.shared.clone(), self.commands.clone());
        }
    }

    /// 解除配对。只影响本机名单，对方再连会被拒。
    pub fn unpair(&self, device_id: &str) -> Result<(), SyncError> {
        lock(&self.shared.book).remove(device_id)
    }

    pub fn device_id(&self) -> String {
        self.shared.identity.device_id()
    }
}

impl Drop for SyncService {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for handle in self.threads.drain(..) {
            let _ = handle.join();
        }
    }
}

fn join(shared: &Shared, address: SocketAddr, code: &str) -> Result<String, SyncError> {
    let stream = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT)?;
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    let mut channel = pair_join(stream, &shared.identity, code)?;
    let own = PairHello {
        name: shared.name.clone(),
        port: shared.port,
    };
    channel.send(&own.encode()?)?;
    let hello = PairHello::decode(&channel.receive()?)?;
    lock(&shared.book).add(
        channel.remote(),
        hello.name.clone(),
        Some(hello.address_from(address)),
    )?;
    shared.record_link(&device_id_of(channel.remote()), true);
    Ok(hello.name)
}

fn spawn_accept(
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

fn spawn_discovery(
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

fn apply_found(shared: &Shared, commands: &Sender<Command>, found: Found) {
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

fn spawn_worker(
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

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(crate) fn device_id_of(public: &[u8; 32]) -> String {
    hex(public)
}
