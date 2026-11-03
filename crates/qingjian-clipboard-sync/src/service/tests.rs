use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{PeerStatus, SyncEvent, SyncOptions, SyncService};
use crate::error::SyncError;
use crate::record::{Record, Stamp};
use crate::sync::Replica;

type Items = Arc<Mutex<BTreeMap<String, Record>>>;

struct Memory(Items);

impl Replica for Memory {
    fn stamps(&self) -> Result<Vec<Stamp>, SyncError> {
        let items = self.0.lock().unwrap();
        Ok(items.values().map(|record| record.stamp.clone()).collect())
    }

    fn record(&self, id: &str) -> Result<Option<Record>, SyncError> {
        Ok(self.0.lock().unwrap().get(id).cloned())
    }

    fn apply(&mut self, record: Record) -> Result<(), SyncError> {
        self.0
            .lock()
            .unwrap()
            .insert(record.stamp.id.clone(), record);
        Ok(())
    }
}

struct Device {
    service: SyncService,

    items: Items,

    events: Receiver<SyncEvent>,

    directory: PathBuf,
}

fn device(tag: &str, name: &str) -> Device {
    let directory =
        std::env::temp_dir().join(format!("qj-sync-svc-{tag}-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    let items = Items::default();
    let (sender, events) = channel();
    let service = SyncService::start(
        SyncOptions {
            directory: directory.clone(),
            name: name.to_owned(),
            model: String::new(),
            own_discovery: true,
        },
        Box::new(Memory(items.clone())),
        sender,
    )
    .unwrap();
    Device {
        service,
        items,
        events,
        directory,
    }
}

fn wait_for(device: &Device, wanted: impl Fn(&SyncEvent) -> bool) -> SyncEvent {
    loop {
        let event = device
            .events
            .recv_timeout(Duration::from_secs(15))
            .expect("event in time");
        if wanted(&event) {
            return event;
        }
    }
}

fn address(device: &Device) -> std::net::SocketAddr {
    ([127, 0, 0, 1], device.service.port()).into()
}

fn add(device: &Device, record: Record) {
    device
        .items
        .lock()
        .unwrap()
        .insert(record.stamp.id.clone(), record);
    device.service.notify_changed();
}

// 端口和 mDNS 是真的，只有地址走回环，所以一次只跑一组
#[test]
fn pair_push_and_refuse_after_unpair() {
    let host = device("flow", "电脑");
    let phone = device("flow", "手机");

    // 输错一次，码就作废
    let code = host.service.begin_pairing().unwrap();
    let wrong = if code == "000000" { "111111" } else { "000000" };
    phone.service.join_pairing(address(&host), wrong.to_owned());
    wait_for(&phone, |event| *event == SyncEvent::PairFailed);
    wait_for(&host, |event| *event == SyncEvent::PairFailed);
    assert!(host.service.peers().is_empty());
    assert!(phone.service.peers().is_empty());
    phone.service.join_pairing(address(&host), code);
    wait_for(&phone, |event| *event == SyncEvent::PairFailed);

    let code = host.service.begin_pairing().unwrap();
    phone.service.join_pairing(address(&host), code);
    let paired = wait_for(&phone, |event| matches!(event, SyncEvent::Paired { .. }));
    assert_eq!(
        paired,
        SyncEvent::Paired {
            name: "电脑".to_owned()
        }
    );
    wait_for(&host, |event| matches!(event, SyncEvent::Paired { .. }));
    assert_eq!(host.service.peers()[0].name, "手机");

    // 配对完马上算已连接，不等 mDNS
    assert_eq!(phone.service.peer_states()[0].status, PeerStatus::Connected);
    assert_eq!(host.service.peer_states()[0].status, PeerStatus::Connected);

    // 不配对也能问到对方是谁
    let info = super::scan::probe(address(&host)).unwrap();
    assert_eq!(info.name, "电脑");
    assert_eq!(info.id, host.service.device_id());

    // 手机复制的传到电脑，电脑亮灯用的事件齐全
    let record = Record::text("from phone".to_owned(), 100);
    add(&phone, record.clone());
    wait_for(
        &host,
        |event| matches!(event, SyncEvent::ReceiveStarted { peer } if peer == "手机"),
    );
    let finished = wait_for(&host, |event| {
        matches!(event, SyncEvent::ReceiveFinished { .. })
    });
    let SyncEvent::ReceiveFinished { newest, .. } = finished else {
        unreachable!();
    };
    assert_eq!(newest.unwrap().id, record.stamp.id);
    assert!(host.items.lock().unwrap().contains_key(&record.stamp.id));

    // 电脑解除配对后，手机再推过不去
    // 先等排队的那几轮跑完：已经握完手的一轮不会被解除配对打断
    std::thread::sleep(Duration::from_millis(800));
    let phone_id = phone.service.device_id();
    // 只看解除之后的事件：前面几轮在机器忙时也可能有一次发送失败，与解除配对无关
    while phone.events.try_recv().is_ok() {}
    host.service.unpair(&phone_id).unwrap();
    add(&phone, Record::text("after unpair".to_owned(), 200));
    std::thread::sleep(Duration::from_secs(2));
    assert_eq!(host.items.lock().unwrap().len(), 1);

    // 手机被拒时知道是对方取消了配对：自己也删掉电脑，不当成发送失败亮红灯
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    while !phone.service.peers().is_empty() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(phone.service.peers().is_empty());
    assert!(
        phone
            .events
            .try_iter()
            .all(|event| !matches!(event, SyncEvent::SendFailed { .. }))
    );

    let directories = [host.directory.clone(), phone.directory.clone()];
    drop((host, phone));
    for directory in directories {
        let _ = std::fs::remove_dir_all(directory);
    }
}
