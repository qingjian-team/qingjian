//! 一轮同步：两边交换清单，各自要对方更新的那些。一问一答轮流说话，双方不会同时写满缓冲区而卡死。
//! 不保持长连接：本机有变化或发现对方上线时连一次，手机后台被杀也不留状态。

#[cfg(test)]
mod tests;

mod progress;
mod replica;
mod wire;

pub use progress::Progress;
pub use replica::Replica;

use std::collections::HashMap;
use std::io::{Read, Write};

use crate::channel::{NOT_PAIRED, SecureChannel};
use crate::error::SyncError;
use crate::record::Stamp;
use wire::{decode_record, encode_record};

/// 主动发起的一方跑一轮，返回收到并记下了几条。
pub fn run_initiator<S: Read + Write>(
    channel: &mut SecureChannel<S>,
    replica: &mut dyn Replica,
    observer: &mut dyn FnMut(Progress),
) -> Result<usize, SyncError> {
    let local = replica.stamps()?;
    channel.send(&serde_json::to_vec(&local)?)?;
    let first = channel.receive()?;
    if first == NOT_PAIRED {
        return Err(SyncError::NotPaired);
    }
    let remote: Vec<Stamp> = serde_json::from_slice(&first)?;
    let they_want: Vec<String> = serde_json::from_slice(&channel.receive()?)?;

    send_records(channel, replica, &they_want)?;
    let want = wanted(&local, &remote);
    channel.send(&serde_json::to_vec(&want)?)?;
    receive_records(channel, replica, &remote, &want, observer)
}

/// 接受连接的一方跑一轮，返回收到并记下了几条。
pub fn run_responder<S: Read + Write>(
    channel: &mut SecureChannel<S>,
    replica: &mut dyn Replica,
    observer: &mut dyn FnMut(Progress),
) -> Result<usize, SyncError> {
    let remote: Vec<Stamp> = serde_json::from_slice(&channel.receive()?)?;
    let local = replica.stamps()?;
    let want = wanted(&local, &remote);
    channel.send(&serde_json::to_vec(&local)?)?;
    channel.send(&serde_json::to_vec(&want)?)?;

    let applied = receive_records(channel, replica, &remote, &want, observer)?;
    let they_want: Vec<String> = serde_json::from_slice(&channel.receive()?)?;
    send_records(channel, replica, &they_want)?;
    Ok(applied)
}

/// `remote` 里比本机新或本机没有的 id。本机没有的删除标记也要：别处删了的以后不该再被别的设备带回来。
fn wanted(local: &[Stamp], remote: &[Stamp]) -> Vec<String> {
    let local: HashMap<&str, &Stamp> = local
        .iter()
        .map(|stamp| (stamp.id.as_str(), stamp))
        .collect();
    remote
        .iter()
        .filter(|stamp| !stamp.forgotten)
        .filter(|stamp| {
            local
                .get(stamp.id.as_str())
                .is_none_or(|own| stamp.newer_than(own))
        })
        .map(|stamp| stamp.id.clone())
        .collect()
}

fn send_records<S: Read + Write>(
    channel: &mut SecureChannel<S>,
    replica: &dyn Replica,
    ids: &[String],
) -> Result<(), SyncError> {
    for id in ids {
        // 对方要的这条在这期间可能被裁掉了，发空消息占位，轮次才对得上
        let message = match replica.record(id)? {
            Some(record) => encode_record(&record)?,
            None => Vec::new(),
        };
        channel.send(&message)?;
    }
    Ok(())
}

fn receive_records<S: Read + Write>(
    channel: &mut SecureChannel<S>,
    replica: &mut dyn Replica,
    remote: &[Stamp],
    want: &[String],
    observer: &mut dyn FnMut(Progress),
) -> Result<usize, SyncError> {
    if !want.is_empty() {
        observer(Progress::Receiving);
    }
    let mut applied = 0;
    for id in want {
        let message = channel.receive_with(&mut |received, total| {
            observer(Progress::Bytes { received, total });
        })?;
        if message.is_empty() {
            continue;
        }
        let record = decode_record(&message)?;
        let announced = remote.iter().any(|stamp| &stamp.id == id);
        if &record.stamp.id != id || !announced || !record.is_consistent() {
            return Err(SyncError::Protocol("record does not match".to_owned()));
        }
        replica.apply(record)?;
        applied += 1;
    }
    Ok(applied)
}
