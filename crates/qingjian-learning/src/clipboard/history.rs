//! [`ClipboardHistory`]：读写、去重、限量，图片文件的存与删。

use std::path::{Path, PathBuf};

use qingjian_core::storage::{read_text_lossy, write_atomic_str};

use super::ClipboardEntry;
use crate::error::LearningError;

pub const CLIPBOARD_FILE: &str = "clipboard.jsonl";

/// 图片存放的子目录（数据目录下）。
pub const CLIPBOARD_IMAGE_DIR: &str = "clipboard-images";

/// 缺省最多存多少条，可在设置里改（[`ClipboardHistory::set_limit`]）。
pub const DEFAULT_CLIPBOARD_LIMIT: usize = 1000;

/// 一条文字最多存多少字符；再长的截断（整篇文章复制进来不该撑大文件）。
const MAX_TEXT_CHARS: usize = 20_000;

#[derive(Debug)]
pub struct ClipboardHistory {
    /// 新的在前。
    entries: Vec<ClipboardEntry>,

    limit: usize,

    /// 数据目录；`None` 只在内存里记（图片也不存）。
    dir: Option<PathBuf>,
}

impl Default for ClipboardHistory {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            limit: DEFAULT_CLIPBOARD_LIMIT,
            dir: None,
        }
    }
}

impl ClipboardHistory {
    /// 读数据目录里的历史（不存在就从零开始）；读不了就退回只在内存里记。
    pub fn open(dir: &Path) -> Self {
        let path = dir.join(CLIPBOARD_FILE);
        match read_text_lossy(&path) {
            Ok(text) => Self {
                entries: text.as_deref().map(parse).unwrap_or_default(),
                limit: DEFAULT_CLIPBOARD_LIMIT,
                dir: Some(dir.to_owned()),
            },
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "剪贴板历史读不了，本次只在内存里记");
                Self::default()
            }
        }
    }

    /// 新的在前。
    pub fn entries(&self) -> &[ClipboardEntry] {
        &self.entries
    }

    pub fn limit(&self) -> usize {
        self.limit
    }

    /// 改最多存多少条（至少 1）；已有的超了就丢最旧的。
    pub fn set_limit(&mut self, limit: usize) -> Result<(), LearningError> {
        self.limit = limit.max(1);
        if self.entries.len() > self.limit {
            self.trim();
            self.save()?;
        }
        Ok(())
    }

    /// 图片文件的完整路径；不是图片条目或只在内存里时为 `None`。
    pub fn image_path(&self, entry: &ClipboardEntry) -> Option<PathBuf> {
        Some(
            self.dir
                .as_ref()?
                .join(CLIPBOARD_IMAGE_DIR)
                .join(entry.image.as_ref()?),
        )
    }

    /// 记一次复制文字（`at` 是 Unix 秒）：原样存，空白的不记；已有同样文字的挪到最前。返回有没有变。
    pub fn add_at(&mut self, text: &str, at: i64) -> Result<bool, LearningError> {
        if text.trim().is_empty() {
            return Ok(false);
        }
        let text: String = text.chars().take(MAX_TEXT_CHARS).collect();
        self.push(ClipboardEntry {
            text,
            image: None,
            at,
        })
    }

    /// 记一次现在复制的文字。
    pub fn add(&mut self, text: &str) -> Result<bool, LearningError> {
        self.add_at(text, now())
    }

    /// 记一次复制图片：`bytes` 是图片文件内容，`extension` 是扩展名（`png`、`jpg`）。按内容取名，同一张挪到最前。
    /// 只在内存里记时不存图片。
    pub fn add_image_at(
        &mut self,
        bytes: &[u8],
        extension: &str,
        at: i64,
    ) -> Result<bool, LearningError> {
        let Some(dir) = &self.dir else {
            return Ok(false);
        };
        if bytes.is_empty() {
            return Ok(false);
        }
        let extension: String = extension
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .take(5)
            .collect();
        let name = format!(
            "{:016x}.{}",
            fnv1a(bytes),
            if extension.is_empty() {
                "png"
            } else {
                &extension
            }
        );
        let images = dir.join(CLIPBOARD_IMAGE_DIR);
        std::fs::create_dir_all(&images)?;
        let path = images.join(&name);
        if !path.is_file() {
            std::fs::write(&path, bytes)?;
        }
        self.push(ClipboardEntry {
            text: String::new(),
            image: Some(name),
            at,
        })
    }

    pub fn add_image(&mut self, bytes: &[u8], extension: &str) -> Result<bool, LearningError> {
        self.add_image_at(bytes, extension, now())
    }

    /// 删掉第 `index` 条（新的在前数）；越界不动。
    pub fn remove(&mut self, index: usize) -> Result<(), LearningError> {
        if index >= self.entries.len() {
            return Ok(());
        }
        let removed = self.entries.remove(index);
        self.forget_image(&removed);
        self.save()
    }

    pub fn clear(&mut self) -> Result<(), LearningError> {
        for entry in std::mem::take(&mut self.entries) {
            self.forget_image(&entry);
        }
        self.save()
    }

    /// 放到最前：同一条（同样的文字或同一张图）先拿掉，再按上限丢最旧的。
    fn push(&mut self, entry: ClipboardEntry) -> Result<bool, LearningError> {
        let same = |other: &ClipboardEntry| other.text == entry.text && other.image == entry.image;
        if self.entries.first().is_some_and(same) {
            return Ok(false);
        }
        self.entries.retain(|other| !same(other));
        self.entries.insert(0, entry);
        self.trim();
        self.save()?;
        Ok(true)
    }

    fn trim(&mut self) {
        while self.entries.len() > self.limit {
            if let Some(dropped) = self.entries.pop() {
                self.forget_image(&dropped);
            }
        }
    }

    /// 条目拿掉之后，没别的条目用这张图就删文件（删不掉只记日志，留个孤儿文件不碍事）。
    fn forget_image(&self, removed: &ClipboardEntry) {
        let Some(name) = &removed.image else {
            return;
        };
        if self
            .entries
            .iter()
            .any(|entry| entry.image.as_ref() == Some(name))
        {
            return;
        }
        if let Some(path) = self.image_path(removed)
            && let Err(error) = std::fs::remove_file(&path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(path = %path.display(), %error, "剪贴板图片删不掉");
        }
    }

    fn save(&self) -> Result<(), LearningError> {
        let Some(dir) = &self.dir else {
            return Ok(());
        };
        let text: String = self
            .entries
            .iter()
            .map(|entry| serde_json::to_string(entry).expect("剪贴板条目总能序列化") + "\n")
            .collect();
        write_atomic_str(&dir.join(CLIPBOARD_FILE), &text)?;
        Ok(())
    }
}

fn now() -> i64 {
    jiff::Timestamp::now().as_second()
}

/// 图片按内容取名用的 64 位 FNV-1a：跨版本、跨平台稳定（标准库的哈希不保证）。
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// 坏行跳过。
fn parse(text: &str) -> Vec<ClipboardEntry> {
    text.lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("qj-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn texts(history: &ClipboardHistory) -> Vec<String> {
        history
            .entries()
            .iter()
            .map(|entry| entry.text.clone())
            .collect()
    }

    #[test]
    fn keeps_text_verbatim_newest_first_without_duplicates() {
        let dir = temp_dir("clipboard-order");
        let mut history = ClipboardHistory::open(&dir);
        let code = "fn main() {\n\tprintln!(\"hi\");\n}\n";
        assert!(history.add_at("一", 1).unwrap());
        assert!(history.add_at(code, 2).unwrap());
        assert!(!history.add_at(code, 3).unwrap());
        assert!(history.add_at("一", 4).unwrap());
        assert!(!history.add_at("  \n", 5).unwrap());
        assert_eq!(texts(&ClipboardHistory::open(&dir)), ["一", code]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn limit_drops_oldest() {
        let dir = temp_dir("clipboard-limit");
        let mut history = ClipboardHistory::open(&dir);
        for (i, text) in ["a", "b", "c"].into_iter().enumerate() {
            history.add_at(text, i as i64).unwrap();
        }
        history.set_limit(2).unwrap();
        assert_eq!(texts(&history), ["c", "b"]);
        history.add_at("d", 9).unwrap();
        assert_eq!(texts(&ClipboardHistory::open(&dir)), ["d", "c"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn images_are_stored_once_and_deleted_with_their_entry() {
        let dir = temp_dir("clipboard-image");
        let mut history = ClipboardHistory::open(&dir);
        assert!(history.add_image_at(b"\x89PNG fake", "png", 1).unwrap());
        history.add_at("文字", 2).unwrap();
        assert!(history.add_image_at(b"\x89PNG fake", "png", 3).unwrap());
        assert_eq!(history.entries().len(), 2);
        let path = history.image_path(&history.entries()[0]).unwrap();
        assert!(path.is_file());
        history.remove(0).unwrap();
        assert!(!path.exists());
        assert_eq!(texts(&ClipboardHistory::open(&dir)), ["文字"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn skips_bad_lines() {
        assert_eq!(parse("{\"text\":\"a\",\"at\":1}\nbroken\n").len(), 1);
    }
}
