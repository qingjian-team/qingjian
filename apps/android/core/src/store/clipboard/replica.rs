//! [`HistoryReplica`]：同步看到的安卓剪贴板历史。历史文件只有文字、图片和秒级时刻，同步要的 id、毫秒和删除标记记在旁边的状态文件里。
//! 面板和监听是直接改历史文件的，所以每次对账：状态里有、历史里没了的，就是本机不要了（裁掉或清空），不传给别的设备。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use qingjian_clipboard_sync::record::content_id;
use qingjian_clipboard_sync::{Record, RecordKind, Replica, Stamp, SyncError};
use qingjian_learning::{ClipboardEntry, ClipboardHistory, MAX_CLIPBOARD_TEXT_CHARS};
use serde::{Deserialize, Serialize};

use super::LOCK;

const STATE_FILE: &str = "clipboard-sync-state.json";

/// 状态文件最多记多少条，再旧的不再比较。
const MAX_STATE: usize = 5000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Status {
    Live,
    Deleted,
    Forgotten,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Known {
    /// 毫秒。
    at: i64,

    status: Status,

    kind: RecordKind,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    items: HashMap<String, Known>,

    /// 图片文件名到 id：id 要读整张图算，算过就记下。文件名按内容取，不会变。
    image_ids: HashMap<String, String>,
}

pub struct HistoryReplica {
    directory: PathBuf,

    limit: usize,
}

fn storage(error: impl std::fmt::Display) -> SyncError {
    SyncError::Storage(error.to_string())
}

fn now_millis() -> i64 {
    jiff::Timestamp::now().as_millisecond()
}

fn load_state(directory: &Path) -> State {
    std::fs::read(directory.join(STATE_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save_state(directory: &Path, state: &State) -> Result<(), SyncError> {
    let bytes = serde_json::to_vec(state)?;
    let path = directory.join(STATE_FILE);
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, bytes).map_err(storage)?;
    std::fs::rename(&temporary, &path).map_err(storage)
}

fn entry_id(
    history: &ClipboardHistory,
    entry: &ClipboardEntry,
    state: &mut State,
) -> Option<String> {
    let Some(name) = &entry.image else {
        return Some(content_id(RecordKind::Text, entry.text.as_bytes()));
    };
    if let Some(id) = state.image_ids.get(name) {
        return Some(id.clone());
    }
    let bytes = std::fs::read(history.image_path(entry)?).ok()?;
    let id = content_id(RecordKind::Image, &bytes);
    state.image_ids.insert(name.clone(), id.clone());
    Some(id)
}

/// 用户在面板里删掉第 `index` 条之前调：这才是要传给别的设备的删除。
pub(super) fn mark_deleted(directory: &Path, index: usize) {
    let history = ClipboardHistory::open(directory);
    let mut state = load_state(directory);
    let Some(entry) = history.entries().get(index) else {
        return;
    };
    let Some(id) = entry_id(&history, entry, &mut state) else {
        return;
    };
    state.items.insert(
        id,
        Known {
            at: now_millis(),
            status: Status::Deleted,
            kind: if entry.image.is_some() {
                RecordKind::Image
            } else {
                RecordKind::Text
            },
        },
    );
    if let Err(error) = save_state(directory, &state) {
        tracing::warn!(%error, "clipboard sync state not saved");
    }
}

impl HistoryReplica {
    pub fn new(directory: &Path, limit: usize) -> Self {
        Self {
            directory: directory.to_owned(),
            limit,
        }
    }

    fn open(&self) -> Result<ClipboardHistory, SyncError> {
        let mut history = ClipboardHistory::open(&self.directory);
        history.set_limit(self.limit).map_err(storage)?;
        Ok(history)
    }

    /// 让状态和历史文件对上。
    fn reconcile(&self, history: &ClipboardHistory, state: &mut State) {
        let mut live = HashMap::new();
        for entry in history.entries() {
            let Some(id) = entry_id(history, entry, state) else {
                continue;
            };
            let previous = state.items.get(&id);
            // 记的毫秒和历史的秒对得上才算数，对不上是本机后来又复制了一次
            let at = previous
                .filter(|known| known.at.div_euclid(1000) == entry.at)
                .map_or(entry.at * 1000, |known| known.at);
            // 面板刚删、历史还没写完的那一刻不会走到这里：删除和对账都在同一把锁里
            let kind = if entry.image.is_some() {
                RecordKind::Image
            } else {
                RecordKind::Text
            };
            live.insert(
                id,
                Known {
                    at,
                    status: Status::Live,
                    kind,
                },
            );
        }
        let now = now_millis();
        for (id, known) in &mut state.items {
            if known.status == Status::Live && !live.contains_key(id) {
                known.status = Status::Forgotten;
                known.at = now;
            }
        }
        state.items.extend(live);
        if state.items.len() > MAX_STATE {
            let mut order: Vec<_> = state
                .items
                .iter()
                .map(|(id, known)| (known.at, id.clone()))
                .collect();
            order.sort();
            for (_, id) in order.iter().take(order.len() - MAX_STATE) {
                state.items.remove(id);
            }
        }
    }

    fn set_status(&self, state: &mut State, record: &Record, status: Status) {
        state.items.insert(
            record.stamp.id.clone(),
            Known {
                at: record.stamp.at,
                status,
                kind: record.kind,
            },
        );
    }
}

impl Replica for HistoryReplica {
    fn stamps(&self) -> Result<Vec<Stamp>, SyncError> {
        let _guard = LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let history = self.open()?;
        let mut state = load_state(&self.directory);
        self.reconcile(&history, &mut state);
        save_state(&self.directory, &state)?;
        Ok(state
            .items
            .iter()
            .map(|(id, known)| Stamp {
                id: id.clone(),
                at: known.at,
                deleted: known.status != Status::Live,
                forgotten: known.status == Status::Forgotten,
            })
            .collect())
    }

    fn record(&self, id: &str) -> Result<Option<Record>, SyncError> {
        let _guard = LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let history = self.open()?;
        let mut state = load_state(&self.directory);
        self.reconcile(&history, &mut state);
        let Some(known) = state.items.get(id).cloned() else {
            return Ok(None);
        };
        if known.status != Status::Live {
            let stamp = Stamp {
                id: id.to_owned(),
                at: known.at,
                deleted: true,
                forgotten: known.status == Status::Forgotten,
            };
            return Ok(Some(Record::tombstone(stamp, known.kind)));
        }
        for entry in history.entries() {
            if entry_id(&history, entry, &mut state).as_deref() != Some(id) {
                continue;
            }
            let Some(name) = &entry.image else {
                return Ok(Some(Record::text(entry.text.clone(), known.at)));
            };
            let Some(path) = history.image_path(entry) else {
                return Ok(None);
            };
            let bytes = std::fs::read(path).map_err(storage)?;
            let extension = name.rsplit('.').next().unwrap_or("png").to_owned();
            return Ok(Some(Record::image(bytes, extension, known.at)));
        }
        Ok(None)
    }

    fn apply(&mut self, record: Record) -> Result<(), SyncError> {
        let _guard = LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let mut history = self.open()?;
        let mut state = load_state(&self.directory);
        self.reconcile(&history, &mut state);
        let seconds = record.stamp.at.div_euclid(1000);
        if record.stamp.deleted {
            let position = (0..history.entries().len()).find(|index| {
                let entry = history.entries()[*index].clone();
                entry_id(&history, &entry, &mut state).as_deref() == Some(&record.stamp.id)
            });
            if let Some(index) = position {
                history.remove(index).map_err(storage)?;
            }
            self.set_status(&mut state, &record, Status::Deleted);
            return save_state(&self.directory, &state);
        }
        let stored = match record.kind {
            RecordKind::Text if record.text.chars().count() > MAX_CLIPBOARD_TEXT_CHARS => false,
            RecordKind::Text => history
                .merge_text_at(&record.text, seconds)
                .map_err(storage)?,
            RecordKind::Image => history
                .merge_image_at(&record.image, &record.extension, seconds)
                .map_err(storage)?,
        };
        // 太长存不下的记成不要了：截断后 id 对不上，不这样每一轮都会再要一次
        let status = if stored {
            Status::Live
        } else {
            Status::Forgotten
        };
        self.set_status(&mut state, &record, status);
        save_state(&self.directory, &state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("qj-android-sync-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn stamp_of<'a>(stamps: &'a [Stamp], text: &str) -> &'a Stamp {
        let id = content_id(RecordKind::Text, text.as_bytes());
        stamps.iter().find(|stamp| stamp.id == id).unwrap()
    }

    #[test]
    fn received_millis_survive_the_second_only_history() {
        let dir = folder("millis");
        let mut replica = HistoryReplica::new(&dir, 10);
        let record = Record::text("from pc".into(), 1_700_000_000_123);
        replica.apply(record.clone()).unwrap();
        let stamps = replica.stamps().unwrap();
        assert_eq!(stamps, vec![record.stamp.clone()]);
        assert_eq!(replica.record(&record.stamp.id).unwrap(), Some(record));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn older_items_do_not_jump_to_the_top() {
        let dir = folder("order");
        let mut replica = HistoryReplica::new(&dir, 10);
        replica.apply(Record::text("new".into(), 9_000)).unwrap();
        replica.apply(Record::text("old".into(), 1_000)).unwrap();
        let history = ClipboardHistory::open(&dir);
        assert_eq!(history.entries()[0].text, "new");
        assert_eq!(history.entries()[1].text, "old");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn panel_delete_spreads_and_clear_only_forgets() {
        let dir = folder("delete");
        let replica = HistoryReplica::new(&dir, 10);
        let mut history = ClipboardHistory::open(&dir);
        history.add_at("gone", 1).unwrap();
        history.add_at("kept", 2).unwrap();
        replica.stamps().unwrap();

        mark_deleted(&dir, 1);
        history.remove(1).unwrap();
        let stamps = replica.stamps().unwrap();
        let gone = stamp_of(&stamps, "gone");
        assert!(gone.deleted && !gone.forgotten);

        history.clear().unwrap();
        let stamps = replica.stamps().unwrap();
        let kept = stamp_of(&stamps, "kept");
        assert!(kept.deleted && kept.forgotten);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn copying_again_after_a_delete_brings_it_back() {
        let dir = folder("again");
        let mut replica = HistoryReplica::new(&dir, 10);
        let record = Record::text("x".into(), 1_000);
        replica.apply(record.clone()).unwrap();
        replica
            .apply(Record::tombstone(
                Stamp {
                    at: 2_000,
                    ..record.stamp.clone()
                },
                RecordKind::Text,
            ))
            .unwrap();
        assert!(ClipboardHistory::open(&dir).entries().is_empty());
        replica.apply(Record::text("x".into(), 3_000)).unwrap();
        let stamps = replica.stamps().unwrap();
        assert!(!stamp_of(&stamps, "x").deleted);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn images_round_trip_with_their_bytes() {
        let dir = folder("image");
        let mut replica = HistoryReplica::new(&dir, 10);
        let record = Record::image(vec![1, 2, 3], "jpg".into(), 5_000);
        replica.apply(record.clone()).unwrap();
        assert_eq!(replica.record(&record.stamp.id).unwrap(), Some(record));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn text_too_long_to_store_is_not_asked_for_again() {
        let dir = folder("long");
        let mut replica = HistoryReplica::new(&dir, 10);
        let record = Record::text("好".repeat(MAX_CLIPBOARD_TEXT_CHARS + 1), 1_000);
        replica.apply(record.clone()).unwrap();
        assert!(ClipboardHistory::open(&dir).entries().is_empty());
        let stamps = replica.stamps().unwrap();
        assert!(stamps[0].forgotten);
        assert!(!record.stamp.newer_than(&stamps[0]));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
