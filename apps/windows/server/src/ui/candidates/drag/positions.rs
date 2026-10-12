//! 游戏候选位置单独保存；存窗口内的万分比坐标，分辨率和显示器原点变化后仍可恢复。

use std::collections::BTreeMap;
use std::io;
use std::path::PathBuf;
use toml_edit::{Array, DocumentMut, Item, Table};
use windows::Win32::Foundation::{POINT, RECT};

pub(super) struct Positions {
    path: Option<PathBuf>,

    values: BTreeMap<String, (i32, i32)>,

    /// 无法解析已有文件时保持原文件，只在本次运行中记位置。
    writable: bool,
}

impl Positions {
    pub(super) fn open(path: Option<PathBuf>) -> Self {
        let mut store = Self {
            path,
            values: BTreeMap::new(),
            writable: true,
        };
        if let Some(path) = &store.path {
            match std::fs::read_to_string(path) {
                Ok(text) => match text.parse::<DocumentMut>() {
                    Ok(doc) => {
                        if let Some(table) = doc.get("positions").and_then(Item::as_table) {
                            for (app, value) in table.iter() {
                                if let Some(array) = value.as_array().filter(|a| a.len() == 2)
                                    && let Some(x) = array.get(0).and_then(|v| v.as_integer())
                                    && let Some(y) = array.get(1).and_then(|v| v.as_integer())
                                    && (0..=10_000).contains(&x)
                                    && (0..=10_000).contains(&y)
                                {
                                    store
                                        .values
                                        .insert(app.to_ascii_lowercase(), (x as i32, y as i32));
                                }
                            }
                        }
                    }
                    Err(_) => store.writable = false,
                },
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(_) => store.writable = false,
            }
        }
        store
    }

    pub(super) fn get(&self, app: &str, bounds: RECT) -> Option<POINT> {
        let &(x, y) = self.values.get(&app.to_ascii_lowercase())?;
        Some(POINT {
            x: bounds.left + ((bounds.right - bounds.left) as i64 * x as i64 / 10_000) as i32,
            y: bounds.top + ((bounds.bottom - bounds.top) as i64 * y as i64 / 10_000) as i32,
        })
    }

    pub(super) fn set(&mut self, app: &str, point: POINT, bounds: RECT) {
        let x = ((point.x - bounds.left) as i64 * 10_000
            / (bounds.right - bounds.left).max(1) as i64)
            .clamp(0, 10_000) as i32;
        let y = ((point.y - bounds.top) as i64 * 10_000
            / (bounds.bottom - bounds.top).max(1) as i64)
            .clamp(0, 10_000) as i32;
        self.values.insert(app.to_ascii_lowercase(), (x, y));
    }

    pub(super) fn save(&self) -> io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if !self.writable {
            return Err(io::Error::other("invalid positions file"));
        }
        let mut doc = DocumentMut::new();
        let mut table = Table::new();
        for (app, &(x, y)) in &self.values {
            let mut array = Array::new();
            array.push(x as i64);
            array.push(y as i64);
            table.insert(app, Item::Value(array.into()));
        }
        doc["positions"] = Item::Table(table);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
        std::fs::write(&temporary, doc.to_string())?;
        std::fs::rename(&temporary, path)
    }
}

#[cfg(test)]
mod tests {
    use super::Positions;
    use windows::Win32::Foundation::{POINT, RECT};

    #[test]
    fn position_scales_and_survives_negative_monitor_origin() {
        let mut store = Positions::open(None);
        store.set(
            "DOTA2.EXE",
            POINT { x: -1440, y: 540 },
            RECT {
                left: -1920,
                top: 0,
                right: 0,
                bottom: 1080,
            },
        );
        let point = store
            .get(
                "dota2.exe",
                RECT {
                    left: 100,
                    top: 50,
                    right: 3540,
                    bottom: 1490,
                },
            )
            .unwrap();
        assert_eq!((point.x, point.y), (960, 770));
        assert!(store.get("other.exe", RECT::default()).is_none());
    }

    #[test]
    fn saves_reloads_and_keeps_other_games() {
        let path = std::env::temp_dir().join(format!(
            "qingjian-drag-{}-{}.toml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_file(&path);
        let bounds = RECT {
            left: 0,
            top: 0,
            right: 1000,
            bottom: 1000,
        };
        let mut store = Positions::open(Some(path.clone()));
        store.set("dota2.exe", POINT { x: 200, y: 300 }, bounds);
        store.set("other.exe", POINT { x: 600, y: 700 }, bounds);
        store.save().unwrap();
        let reloaded = Positions::open(Some(path.clone()));
        assert_eq!(reloaded.get("dota2.exe", bounds).unwrap().x, 200);
        assert_eq!(reloaded.get("other.exe", bounds).unwrap().y, 700);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn broken_positions_file_is_preserved() {
        let path = std::env::temp_dir().join(format!(
            "qingjian-drag-broken-{}-{}.toml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "broken=[").unwrap();
        let store = Positions::open(Some(path.clone()));
        assert!(store.save().is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "broken=[");
        std::fs::remove_file(&path).unwrap();
    }
}
