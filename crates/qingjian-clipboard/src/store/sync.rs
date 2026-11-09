//! 存储的同步一面：每条多一个按内容算的 `sync_id`，各设备各自复制的同样内容才认得出是同一条。
//! 文件不同步（路径到别的设备没有意义），所以没有 `sync_id`。

use std::collections::HashSet;
use std::path::Path;

use qingjian_clipboard_sync::record::content_id;
use qingjian_clipboard_sync::{Record, RecordKind, Replica, Stamp, SyncError};
use rusqlite::{Connection, OptionalExtension, params};

use super::{ClipStore, SCHEMA_VERSION, SELECT_ITEM, read_item};
use crate::capture::{Capture, FORMAT_PNG};
use crate::error::ClipboardError;
use crate::item::ClipItem;
use crate::kind::ClipKind;

/// 别的设备发来的不是 PNG 时，原样留一份：转过的 PNG 算不回原来的 id。
const FORMAT_SYNC_ORIGINAL: &str = "sync_original";

const FORMAT_SYNC_EXTENSION: &str = "sync_extension";

/// 清单最多带多少条，再旧的不再比较。
const MAX_STAMPS: i64 = 5000;

/// 把收到的图片文件转成 `(PNG, 缩略图 PNG)`，解不开为 `None`。窗口层的图片库在壳里，这里不依赖它。
pub type ImageDecoder = fn(&[u8]) -> Option<(Vec<u8>, Vec<u8>)>;

fn id_of(kind: ClipKind, text: &str, png: Option<&[u8]>) -> Option<String> {
    match kind {
        ClipKind::Files => None,
        ClipKind::Image => png.map(|png| content_id(RecordKind::Image, png)),
        _ => Some(content_id(RecordKind::Text, text.as_bytes())),
    }
}

pub(super) fn capture_id(capture: &Capture) -> Option<String> {
    let png = capture
        .formats
        .iter()
        .find(|format| format.name == FORMAT_PNG)
        .map(|format| format.data.as_slice());
    id_of(capture.kind, &capture.text, png)
}

/// 版本 2 的库升上来，已有的历史不能丢。
pub(super) fn migrate_from_v2(connection: &Connection) -> Result<(), ClipboardError> {
    connection.execute_batch(
        "ALTER TABLE items ADD COLUMN forgotten INTEGER NOT NULL DEFAULT 0;
         ALTER TABLE items ADD COLUMN sync_id TEXT;
         CREATE INDEX items_sync ON items (sync_id);",
    )?;
    let rows: Vec<(String, String, String, Option<Vec<u8>>)> = connection
        .prepare(
            "SELECT id, kind, text,
                 (SELECT data FROM formats WHERE item_id = items.id AND name = ?1)
             FROM items WHERE deleted = 0",
        )?
        .query_map([FORMAT_PNG], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .collect::<Result<_, _>>()?;
    for (id, kind, text, png) in rows {
        let sync_id = id_of(ClipKind::parse(&kind), &text, png.as_deref());
        connection.execute(
            "UPDATE items SET sync_id = ?1 WHERE id = ?2",
            params![sync_id, id],
        )?;
    }
    connection.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION}"))?;
    Ok(())
}

/// 内容要改了：给旧内容留一个删除标记，别的设备才知道旧的那条不要了。
pub(super) fn leave_tombstone(
    connection: &Connection,
    id: &str,
    now: i64,
) -> Result<(), ClipboardError> {
    connection.execute(
        "INSERT INTO items
         (id, device_id, kind, text, source, fingerprint, created_at, updated_at, deleted, sync_id)
         SELECT id || '-old-' || ?2, device_id, kind, '', '', '', ?2, ?2, 1, sync_id
         FROM items WHERE id = ?1 AND sync_id IS NOT NULL",
        params![id, now],
    )?;
    Ok(())
}

impl ClipStore {
    /// 按同步 id 找没删的那一条。
    pub fn item_by_sync_id(&self, id: &str) -> Result<Option<ClipItem>, ClipboardError> {
        Ok(self
            .connection
            .query_row(
                &format!("{SELECT_ITEM} WHERE sync_id = ?1 AND deleted = 0 LIMIT 1"),
                [id],
                read_item,
            )
            .optional()?)
    }
}

/// 同步用的那份存储：和面板各开一个连接，SQLite 负责互斥。
pub struct StoreReplica {
    store: ClipStore,

    decode: ImageDecoder,
}

fn storage(error: impl std::fmt::Display) -> SyncError {
    SyncError::Storage(error.to_string())
}

impl StoreReplica {
    pub fn open(path: &Path, limit: usize, decode: ImageDecoder) -> Result<Self, ClipboardError> {
        Ok(Self {
            store: ClipStore::open(path, limit)?,
            decode,
        })
    }

    fn apply_live(&mut self, record: Record) -> Result<(), ClipboardError> {
        let id = record.stamp.id.clone();
        let at = record.stamp.at;
        let connection = &self.store.connection;
        let moved = connection.execute(
            "UPDATE items SET updated_at = ?1 WHERE sync_id = ?2 AND deleted = 0",
            params![at, id],
        )?;
        if moved > 0 {
            return Ok(());
        }
        connection.execute(
            "DELETE FROM items WHERE sync_id = ?1 AND deleted = 1",
            [&id],
        )?;
        let capture = match record.kind {
            RecordKind::Text => Capture::text(record.text, String::new(), Vec::new()),
            RecordKind::Image => {
                let Some((png, thumbnail)) = (self.decode)(&record.image) else {
                    tracing::info!(%id, "received image cannot be decoded");
                    return Ok(());
                };
                let same = png == record.image;
                let capture = Capture::image(png, thumbnail, String::new());
                let computed = capture_id(&capture);
                self.store.add(&capture, at)?;
                if !same {
                    self.keep_original(&computed, &id, &record.image, &record.extension)?;
                }
                return Ok(());
            }
        };
        self.store.add(&capture, at)
    }

    fn keep_original(
        &self,
        computed: &Option<String>,
        id: &str,
        image: &[u8],
        extension: &str,
    ) -> Result<(), ClipboardError> {
        let connection = &self.store.connection;
        connection.execute(
            "UPDATE items SET sync_id = ?1 WHERE sync_id = ?2 AND deleted = 0",
            params![id, computed],
        )?;
        for (name, data) in [
            (FORMAT_SYNC_ORIGINAL, image),
            (FORMAT_SYNC_EXTENSION, extension.as_bytes()),
        ] {
            connection.execute(
                "INSERT OR REPLACE INTO formats (item_id, name, data)
                 SELECT id, ?1, ?2 FROM items WHERE sync_id = ?3 AND deleted = 0",
                params![name, data, id],
            )?;
        }
        Ok(())
    }

    fn apply_deleted(&mut self, record: &Record) -> Result<(), ClipboardError> {
        let id = &record.stamp.id;
        let at = record.stamp.at;
        let transaction = self.store.connection.transaction()?;
        transaction.execute(
            "DELETE FROM formats WHERE item_id IN (SELECT id FROM items WHERE sync_id = ?1)",
            [id],
        )?;
        let changed = transaction.execute(
            "UPDATE items SET text = '', source = '', fingerprint = '', deleted = 1, forgotten = 0,
                 updated_at = ?1
             WHERE sync_id = ?2",
            params![at, id],
        )?;
        if changed == 0 {
            let kind = match record.kind {
                RecordKind::Text => ClipKind::Text,
                RecordKind::Image => ClipKind::Image,
            };
            transaction.execute(
                "INSERT INTO items
                 (id, device_id, kind, text, source, fingerprint, created_at, updated_at, deleted, sync_id)
                 VALUES (?1, ?2, ?3, '', '', '', ?4, ?4, 1, ?5)",
                params![
                    format!("{id}-gone"),
                    self.store.device_id,
                    kind.key(),
                    at,
                    id
                ],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    fn read(&self, id: &str) -> Result<Option<Record>, ClipboardError> {
        let row: Option<(String, String, String, i64, bool)> = self
            .store
            .connection
            .query_row(
                "SELECT id, kind, text, updated_at, deleted FROM items
                 WHERE sync_id = ?1 ORDER BY updated_at DESC LIMIT 1",
                [id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .optional()?;
        let Some((item, kind, text, at, deleted)) = row else {
            return Ok(None);
        };
        let kind = ClipKind::parse(&kind);
        let record_kind = if kind == ClipKind::Image {
            RecordKind::Image
        } else {
            RecordKind::Text
        };
        if deleted {
            let stamp = Stamp {
                id: id.to_owned(),
                at,
                deleted: true,
                forgotten: false,
            };
            return Ok(Some(Record::tombstone(stamp, record_kind)));
        }
        Ok(match record_kind {
            RecordKind::Text => Some(Record::text(text, at)),
            RecordKind::Image => {
                let original = self.store.format(&item, FORMAT_SYNC_ORIGINAL)?;
                let extension = self
                    .store
                    .format(&item, FORMAT_SYNC_EXTENSION)?
                    .and_then(|bytes| String::from_utf8(bytes).ok())
                    .unwrap_or_else(|| "png".to_owned());
                original
                    .or(self.store.format(&item, FORMAT_PNG)?)
                    .map(|image| Record::image(image, extension, at))
            }
        })
    }
}

impl Replica for StoreReplica {
    fn stamps(&self) -> Result<Vec<Stamp>, SyncError> {
        let connection = &self.store.connection;
        let mut statement = connection
            .prepare(
                "SELECT sync_id, updated_at, deleted, forgotten FROM items
                 WHERE sync_id IS NOT NULL ORDER BY updated_at DESC LIMIT ?1",
            )
            .map_err(storage)?;
        let rows = statement
            .query_map([MAX_STAMPS], |row| {
                Ok(Stamp {
                    id: row.get(0)?,
                    at: row.get(1)?,
                    deleted: row.get(2)?,
                    forgotten: row.get(3)?,
                })
            })
            .map_err(storage)?;
        // 同样文字、不同格式的两条 id 相同，只报最新的
        let mut seen = HashSet::new();
        let mut stamps = Vec::new();
        for row in rows {
            let stamp = row.map_err(storage)?;
            if seen.insert(stamp.id.clone()) {
                stamps.push(stamp);
            }
        }
        Ok(stamps)
    }

    fn record(&self, id: &str) -> Result<Option<Record>, SyncError> {
        self.read(id).map_err(storage)
    }

    fn apply(&mut self, record: Record) -> Result<(), SyncError> {
        if record.stamp.deleted {
            self.apply_deleted(&record).map_err(storage)
        } else {
            self.apply_live(record).map_err(storage)
        }
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::kind::ClipFilter;

    fn replica() -> StoreReplica {
        StoreReplica {
            store: ClipStore::with_connection(Connection::open_in_memory().unwrap(), 2).unwrap(),
            decode: |bytes| Some((bytes.to_vec(), vec![0])),
        }
    }

    fn plain(text: &str) -> Capture {
        Capture::text(text.to_owned(), String::new(), Vec::new())
    }

    #[test]
    fn local_copies_and_received_records_share_ids() {
        let mut one = replica();
        one.store.add(&plain("hello"), 5).unwrap();
        let stamp = one.stamps().unwrap().remove(0);
        assert_eq!(stamp.id, Record::text("hello".into(), 5).stamp.id);
        let record = one.record(&stamp.id).unwrap().unwrap();
        assert!(record.is_consistent());

        let mut two = replica();
        two.apply(record).unwrap();
        assert_eq!(two.stamps().unwrap(), vec![stamp]);
    }

    #[test]
    fn user_delete_spreads_but_eviction_only_forgets() {
        let mut store = replica();
        store.store.add(&plain("a"), 1).unwrap();
        store.store.add(&plain("b"), 2).unwrap();
        store.store.add(&plain("c"), 3).unwrap();
        let stamps = store.stamps().unwrap();
        let evicted = stamps
            .iter()
            .find(|stamp| stamp.id == Record::text("a".into(), 0).stamp.id)
            .unwrap();
        assert!(evicted.deleted && evicted.forgotten);

        let item = store
            .store
            .search("b", ClipFilter::All, 1)
            .unwrap()
            .remove(0);
        store.store.delete(&item.id, 9).unwrap();
        let id = Record::text("b".into(), 0).stamp.id;
        let stamps = store.stamps().unwrap();
        let deleted = stamps.iter().find(|stamp| stamp.id == id).unwrap();
        assert!(deleted.deleted && !deleted.forgotten);
        assert!(store.record(&id).unwrap().unwrap().is_consistent());
    }

    #[test]
    fn remote_delete_erases_content_and_newer_copy_comes_back() {
        let mut store = replica();
        store.store.add(&plain("secret"), 1).unwrap();
        let stamp = store.stamps().unwrap().remove(0);
        let gone = Stamp {
            at: 2,
            ..stamp.clone()
        };
        store
            .apply(Record::tombstone(gone, RecordKind::Text))
            .unwrap();
        assert!(
            store
                .store
                .search("", ClipFilter::All, 9)
                .unwrap()
                .is_empty()
        );

        store.apply(Record::text("secret".into(), 3)).unwrap();
        let items = store.store.search("", ClipFilter::All, 9).unwrap();
        assert_eq!(items[0].text, "secret");
        assert_eq!(store.stamps().unwrap().len(), 1);
    }

    #[test]
    fn converted_images_still_send_the_original_bytes() {
        let mut store = replica();
        store.decode = |_| Some((vec![9, 9], vec![1]));
        let jpeg = Record::image(vec![1, 2, 3], "jpg".into(), 4);
        let id = jpeg.stamp.id.clone();
        store.apply(jpeg).unwrap();
        let back = store.record(&id).unwrap().unwrap();
        assert!(back.is_consistent());
        assert_eq!(back.image, vec![1, 2, 3]);
        assert_eq!(back.extension, "jpg");
    }

    #[test]
    fn editing_leaves_a_delete_for_the_old_text() {
        let mut store = replica();
        store.store.add(&plain("old"), 1).unwrap();
        let item = store
            .store
            .search("", ClipFilter::All, 1)
            .unwrap()
            .remove(0);
        store.store.edit_text(&item.id, "new", 2).unwrap();
        let stamps = store.stamps().unwrap();
        let old = Record::text("old".into(), 0).stamp.id;
        let new = Record::text("new".into(), 0).stamp.id;
        assert!(stamps.iter().any(|stamp| stamp.id == old && stamp.deleted));
        assert!(stamps.iter().any(|stamp| stamp.id == new && !stamp.deleted));
    }

    #[test]
    fn v2_databases_keep_their_history() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 CREATE TABLE items (id TEXT PRIMARY KEY, device_id TEXT NOT NULL, kind TEXT NOT NULL,
                     text TEXT NOT NULL, source TEXT NOT NULL, fingerprint TEXT NOT NULL,
                     created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
                     deleted INTEGER NOT NULL DEFAULT 0);
                 CREATE TABLE formats (item_id TEXT NOT NULL, name TEXT NOT NULL, data BLOB NOT NULL,
                     PRIMARY KEY (item_id, name));
                 INSERT INTO items VALUES ('x', 'd', 'text', 'kept', '', 'f', 1, 1, 0);
                 PRAGMA user_version = 2;",
            )
            .unwrap();
        let store = StoreReplica {
            store: ClipStore::with_connection(connection, 10).unwrap(),
            decode: |_| None,
        };
        let items = store.store.search("", ClipFilter::All, 9).unwrap();
        assert_eq!(items[0].text, "kept");
        assert_eq!(
            store.stamps().unwrap()[0].id,
            Record::text("kept".into(), 0).stamp.id
        );
    }
}
