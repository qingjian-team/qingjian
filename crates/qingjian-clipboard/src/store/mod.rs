//! [`ClipStore`]：剪贴板历史存在 SQLite 里。每条有全局 id、设备 id、修改时刻与删除标记；
//! 删除只清空内容、留下标记，同步时别的设备才知道这条删了。

mod sync;
#[cfg(test)]
mod tests;

pub use sync::{ImageDecoder, StoreReplica};

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::capture::{Capture, ClipFormat, FORMAT_PNG, MAX_TEXT_CHARS, text_kind};
use crate::error::ClipboardError;
use crate::item::ClipItem;
use crate::kind::{ClipFilter, ClipKind};

/// 整个历史最多占多少（字节），超了从最旧的删起，最新的一条总留着。
const MAX_STORED_BYTES: i64 = 512 * 1024 * 1024;

/// 表结构的版本。版本 2 的库就地迁移，更旧的直接重建。
const SCHEMA_VERSION: i64 = 3;

const SCHEMA: &str = "
DROP TABLE IF EXISTS items;
DROP TABLE IF EXISTS formats;
DROP TABLE IF EXISTS meta;
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE items (
    id TEXT PRIMARY KEY,
    device_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    text TEXT NOT NULL,
    source TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted INTEGER NOT NULL DEFAULT 0,
    forgotten INTEGER NOT NULL DEFAULT 0,
    sync_id TEXT
);
CREATE TABLE formats (
    item_id TEXT NOT NULL,
    name TEXT NOT NULL,
    data BLOB NOT NULL,
    PRIMARY KEY (item_id, name)
);
CREATE INDEX items_recent ON items (deleted, updated_at DESC);
CREATE INDEX items_fingerprint ON items (fingerprint);
CREATE INDEX items_sync ON items (sync_id);
";

const SELECT_ITEM: &str = "SELECT id, kind, text, source, updated_at FROM items";

pub struct ClipStore {
    connection: Connection,

    device_id: String,

    /// 最多留多少条没删的；超了最旧的按删除处理。
    limit: usize,

    /// 见 [`MAX_STORED_BYTES`]。
    byte_limit: i64,
}

fn read_item(row: &Row) -> rusqlite::Result<ClipItem> {
    let kind: String = row.get(1)?;
    Ok(ClipItem {
        id: row.get(0)?,
        kind: ClipKind::parse(&kind),
        text: row.get(2)?,
        source: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

impl ClipStore {
    pub fn open(path: &Path, limit: usize) -> Result<Self, ClipboardError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Self::with_connection(Connection::open(path)?, limit)
    }

    fn with_connection(connection: Connection, limit: usize) -> Result<Self, ClipboardError> {
        // 删掉的内容要从文件里抹掉，不只是标记成可复用
        connection.execute_batch("PRAGMA secure_delete = ON")?;
        let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version == 2 {
            sync::migrate_from_v2(&connection)?;
        } else if version != SCHEMA_VERSION {
            connection.execute_batch(SCHEMA)?;
            connection.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION}"))?;
        }
        let saved: Option<String> = connection
            .query_row(
                "SELECT value FROM meta WHERE key = 'device_id'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let device_id = match saved {
            Some(id) => id,
            None => {
                let id = format!("{:016x}", RandomState::new().build_hasher().finish());
                connection.execute(
                    "INSERT INTO meta (key, value) VALUES ('device_id', ?1)",
                    [&id],
                )?;
                id
            }
        };
        Ok(Self {
            connection,
            device_id,
            limit: limit.max(1),
            byte_limit: MAX_STORED_BYTES,
        })
    }

    /// 记一次复制。没内容的不记；同样的内容已有就挪到最前。`now` 是 Unix 毫秒。
    pub fn add(&mut self, capture: &Capture, now: i64) -> Result<(), ClipboardError> {
        if capture.is_empty() || capture.text.chars().count() > MAX_TEXT_CHARS {
            return Ok(());
        }
        let text = capture.text.as_str();
        let fingerprint = capture.fingerprint();
        let transaction = self.connection.transaction()?;
        let moved = transaction.execute(
            "UPDATE items SET updated_at = ?1 WHERE fingerprint = ?2 AND deleted = 0",
            params![now, fingerprint],
        )?;
        if moved == 0 {
            let sequence: i64 = transaction.query_row(
                "INSERT INTO meta (key, value) VALUES ('sequence', '1')
                 ON CONFLICT (key) DO UPDATE SET value = CAST(value AS INTEGER) + 1
                 RETURNING CAST(value AS INTEGER)",
                [],
                |row| row.get(0),
            )?;
            let id = format!("{}-{sequence:x}", self.device_id);
            transaction.execute(
                "INSERT INTO items
                 (id, device_id, kind, text, source, fingerprint, created_at, updated_at, sync_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?8)",
                params![
                    id,
                    self.device_id,
                    capture.kind.key(),
                    text,
                    capture.source,
                    fingerprint,
                    now,
                    sync::capture_id(capture)
                ],
            )?;
            for format in &capture.formats {
                transaction.execute(
                    "INSERT INTO formats (item_id, name, data) VALUES (?1, ?2, ?3)",
                    params![id, format.name, format.data],
                )?;
            }
        }
        Self::evict_by_count(&transaction, self.limit, now)?;
        Self::evict_by_size(&transaction, self.byte_limit, now)?;
        transaction.execute(
            "DELETE FROM formats WHERE item_id IN (SELECT id FROM items WHERE deleted = 1)",
            [],
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// 改最大条数；改小了立刻删掉多出来的旧记录，不等下次复制。
    pub fn set_limit(&mut self, limit: usize, now: i64) -> Result<(), ClipboardError> {
        self.limit = limit.max(1);
        let transaction = self.connection.transaction()?;
        Self::evict_by_count(&transaction, self.limit, now)?;
        transaction.execute(
            "DELETE FROM formats WHERE item_id IN (SELECT id FROM items WHERE deleted = 1)",
            [],
        )?;
        transaction.commit()?;
        Ok(())
    }

    fn evict_by_count(
        connection: &Connection,
        limit: usize,
        now: i64,
    ) -> Result<(), ClipboardError> {
        connection.execute(
            "UPDATE items SET text = '', source = '', fingerprint = '', deleted = 1, forgotten = 1, updated_at = ?1
             WHERE deleted = 0 AND id NOT IN (
                 SELECT id FROM items WHERE deleted = 0
                 ORDER BY updated_at DESC, rowid DESC LIMIT ?2
             )",
            params![now, limit as i64],
        )?;
        Ok(())
    }

    /// 总量超了就从最旧的删起，最新的一条不动。
    fn evict_by_size(
        connection: &Connection,
        byte_limit: i64,
        now: i64,
    ) -> Result<(), ClipboardError> {
        let sizes: Vec<(String, i64)> = connection
            .prepare(
                "SELECT id, LENGTH(CAST(text AS BLOB)) + COALESCE(
                     (SELECT SUM(LENGTH(data)) FROM formats WHERE item_id = items.id), 0)
                 FROM items WHERE deleted = 0
                 ORDER BY updated_at ASC, rowid ASC",
            )?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?;
        let mut over = sizes.iter().map(|(_, size)| size).sum::<i64>() - byte_limit;
        for (id, size) in sizes.iter().take(sizes.len().saturating_sub(1)) {
            if over <= 0 {
                break;
            }
            connection.execute(
                "UPDATE items SET text = '', source = '', fingerprint = '', deleted = 1, forgotten = 1, updated_at = ?1
                 WHERE id = ?2",
                params![now, id],
            )?;
            over -= size;
        }
        Ok(())
    }

    /// 没删的条目，最近的在前；`query` 非空时只要包含它的（ASCII 不分大小写）。
    pub fn search(
        &self,
        query: &str,
        filter: ClipFilter,
        limit: usize,
    ) -> Result<Vec<ClipItem>, ClipboardError> {
        let pattern = format!(
            "%{}%",
            query
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        let mut statement = self.connection.prepare_cached(&format!(
            "{SELECT_ITEM}
             WHERE deleted = 0 AND text LIKE ?1 ESCAPE '\\'
               AND (?3 = 'all' OR kind = ?3 OR (?3 = 'text' AND kind = 'rich'))
             ORDER BY updated_at DESC, rowid DESC LIMIT ?2"
        ))?;
        let rows = statement.query_map(params![pattern, limit as i64, filter.key()], read_item)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// 这一条存的所有格式，粘贴时写回系统剪贴板。
    pub fn formats(&self, id: &str) -> Result<Vec<ClipFormat>, ClipboardError> {
        let mut statement = self
            .connection
            .prepare_cached("SELECT name, data FROM formats WHERE item_id = ?1")?;
        let rows = statement.query_map([id], |row| {
            Ok(ClipFormat {
                name: row.get(0)?,
                data: row.get(1)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn format(&self, id: &str, name: &str) -> Result<Option<Vec<u8>>, ClipboardError> {
        Ok(self
            .connection
            .prepare_cached("SELECT data FROM formats WHERE item_id = ?1 AND name = ?2")?
            .query_row([id, name], |row| row.get(0))
            .optional()?)
    }

    /// 这条图片的宽、高（像素）与存储大小（字节）。只取 PNG 文件头与长度，不把整张图读出来。
    pub fn image_info(&self, id: &str) -> Result<Option<(u32, u32, u64)>, ClipboardError> {
        let row: Option<(i64, Vec<u8>)> = self
            .connection
            .prepare_cached(
                "SELECT LENGTH(data), SUBSTR(data, 17, 8) FROM formats
                 WHERE item_id = ?1 AND name = ?2",
            )?
            .query_row([id, FORMAT_PNG], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional()?;
        // PNG 的文件头：8 字节签名、8 字节块头，接着是大端的宽、高
        Ok(row.and_then(|(size, header)| {
            let width = u32::from_be_bytes(header.get(0..4)?.try_into().ok()?);
            let height = u32::from_be_bytes(header.get(4..8)?.try_into().ok()?);
            Some((width, height, size as u64))
        }))
    }

    /// 用过一次：挪到最前。
    pub fn touch(&mut self, id: &str, now: i64) -> Result<(), ClipboardError> {
        self.connection.execute(
            "UPDATE items SET updated_at = ?1 WHERE id = ?2 AND deleted = 0",
            params![now, id],
        )?;
        Ok(())
    }

    /// 改文字。原来的 HTML、RTF 和新文字对不上了，一起丢掉。
    pub fn edit_text(&mut self, id: &str, text: &str, now: i64) -> Result<(), ClipboardError> {
        let text: String = text.chars().take(MAX_TEXT_CHARS).collect();
        let capture = Capture::text(text.clone(), String::new(), Vec::new());
        let transaction = self.connection.transaction()?;
        sync::leave_tombstone(&transaction, id, now)?;
        transaction.execute(
            "UPDATE items SET text = ?1, kind = ?2, fingerprint = ?3, updated_at = ?4, sync_id = ?5
             WHERE id = ?6 AND deleted = 0",
            params![
                text,
                text_kind(&text, false).key(),
                capture.fingerprint(),
                now,
                sync::capture_id(&capture),
                id
            ],
        )?;
        transaction.execute("DELETE FROM formats WHERE item_id = ?1", [id])?;
        transaction.commit()?;
        Ok(())
    }

    /// 全部删掉，留下删除标记；再把释放出来的页收掉，文件不留痕迹。
    pub fn clear(&mut self, now: i64) -> Result<(), ClipboardError> {
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "UPDATE items SET text = '', source = '', fingerprint = '', deleted = 1, forgotten = 1, updated_at = ?1
             WHERE deleted = 0",
            [now],
        )?;
        transaction.execute("DELETE FROM formats", [])?;
        transaction.commit()?;
        self.connection.execute_batch("VACUUM")?;
        Ok(())
    }

    /// 删掉一条：清空内容、留下删除标记。
    pub fn delete(&mut self, id: &str, now: i64) -> Result<(), ClipboardError> {
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "UPDATE items SET text = '', source = '', fingerprint = '', deleted = 1, updated_at = ?1
             WHERE id = ?2",
            params![now, id],
        )?;
        transaction.execute("DELETE FROM formats WHERE item_id = ?1", [id])?;
        transaction.commit()?;
        Ok(())
    }
}
