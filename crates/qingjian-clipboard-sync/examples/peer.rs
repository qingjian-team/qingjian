//! 调试用的同步对端：不碰系统剪贴板，内容只在内存里。用来和真机对测配对与收发。
//! 用法：`cargo run -p qingjian-clipboard-sync --example peer -- <数据目录> <设备名>`，然后在标准输入里敲：
//! `code`（生成配对码）、`join <地址> <码>`、`send <文字>`、`list`、`devices`、`quit`。

use std::collections::BTreeMap;
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use qingjian_clipboard_sync::{Record, Replica, Stamp, SyncError, SyncOptions, SyncService};

type Items = Arc<Mutex<BTreeMap<String, Record>>>;

struct Memory(Items);

impl Replica for Memory {
    fn stamps(&self) -> Result<Vec<Stamp>, SyncError> {
        let items = self.0.lock().expect("not poisoned");
        Ok(items.values().map(|record| record.stamp.clone()).collect())
    }

    fn record(&self, id: &str) -> Result<Option<Record>, SyncError> {
        Ok(self.0.lock().expect("not poisoned").get(id).cloned())
    }

    fn apply(&mut self, record: Record) -> Result<(), SyncError> {
        self.0
            .lock()
            .expect("not poisoned")
            .insert(record.stamp.id.clone(), record);
        Ok(())
    }
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as i64)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let directory = PathBuf::from(args.next().ok_or("expects a data directory")?);
    let name = args.next().ok_or("expects a device name")?;
    let items = Items::default();
    let (sender, events) = channel();
    let service = SyncService::start(
        SyncOptions {
            directory,
            name,
            model: "peer".to_owned(),
            own_discovery: true,
        },
        Box::new(Memory(items.clone())),
        sender,
    )?;
    println!("listening on {}", service.port());
    std::thread::spawn(move || {
        while let Ok(event) = events.recv() {
            println!("event {event:?}");
        }
    });
    for line in std::io::stdin().lock().lines() {
        let line = line?;
        let mut parts = line.splitn(3, ' ');
        match parts.next().unwrap_or_default() {
            "code" => println!("code {}", service.begin_pairing()?),
            "join" => {
                let address = parts.next().ok_or("join <address> <code>")?.parse()?;
                let code = parts.next().ok_or("join <address> <code>")?.to_owned();
                service.join_pairing(address, code);
            }
            "send" => {
                let text = line.trim_start_matches("send ").to_owned();
                let record = Record::text(text, now_millis());
                items
                    .lock()
                    .expect("not poisoned")
                    .insert(record.stamp.id.clone(), record);
                service.notify_changed();
                println!("sent");
            }
            "list" => {
                for record in items.lock().expect("not poisoned").values() {
                    println!(
                        "item deleted={} image={}B text={:?}",
                        record.stamp.deleted,
                        record.image.len(),
                        record.text
                    );
                }
            }
            "devices" => {
                println!("peers {:?}", service.peers());
                println!("nearby {:?}", service.nearby());
            }
            "quit" => break,
            other => println!("unknown command {other}"),
        }
    }
    Ok(())
}
