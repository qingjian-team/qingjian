//! 符号 / 表情面板里每个条目用了几次（`panel-usage.tsv`，一行 `条目\t次数\t最后一次的 Unix 秒`），
//! 给 [`qingjian_core::common_items`] 排「常用」。文件很小，每用一次整个重写（先写临时文件再换名）。

use std::path::{Path, PathBuf};

use qingjian_core::storage::{read_text_lossy, write_atomic_str};
use qingjian_core::{PanelHistory, PanelUse};

use crate::error::LearningError;

pub const PANEL_USAGE_FILE: &str = "panel-usage.tsv";

/// 最多记多少条；超了丢用得最少、最久没用的。
const MAX_ENTRIES: usize = 300;

#[derive(Debug, Default)]
pub struct PanelUsage {
    uses: Vec<PanelUse>,

    /// `None` 只在内存里记。
    path: Option<PathBuf>,
}

impl PanelUsage {
    /// 读数据目录里的记录（不存在就从零开始）；读不了就退回只在内存里记。
    pub fn open(dir: &Path) -> Self {
        let path = dir.join(PANEL_USAGE_FILE);
        match read_text_lossy(&path) {
            Ok(text) => Self {
                uses: text.as_deref().map(parse).unwrap_or_default(),
                path: Some(path),
            },
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "面板使用记录读不了，本次只在内存里记");
                Self::default()
            }
        }
    }

    pub fn uses(&self) -> &[PanelUse] {
        &self.uses
    }

    /// 记一次 `item` 在 `at`（Unix 秒）被用了，当场落盘。
    pub fn record_at(&mut self, item: &str, at: i64) -> Result<(), LearningError> {
        if item.is_empty() || item.contains(['\t', '\n']) {
            return Ok(());
        }
        match self.uses.iter_mut().find(|entry| entry.item == item) {
            Some(entry) => {
                entry.count = entry.count.saturating_add(1);
                entry.last = at;
            }
            None => self.uses.push(PanelUse {
                item: item.to_owned(),
                count: 1,
                last: at,
            }),
        }
        if self.uses.len() > MAX_ENTRIES {
            self.uses
                .sort_by(|a, b| b.count.cmp(&a.count).then(b.last.cmp(&a.last)));
            self.uses.truncate(MAX_ENTRIES);
        }
        self.save()
    }

    /// 记一次现在用的。
    pub fn record(&mut self, item: &str) -> Result<(), LearningError> {
        self.record_at(item, jiff::Timestamp::now().as_second())
    }

    fn save(&self) -> Result<(), LearningError> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let text: String = self
            .uses
            .iter()
            .map(|entry| format!("{}\t{}\t{}\n", entry.item, entry.count, entry.last))
            .collect();
        write_atomic_str(path, &text)?;
        Ok(())
    }
}

/// 给桌面端的 Engine 用：打开符号面板时排「常用」，从面板上屏时记一次。
impl PanelHistory for PanelUsage {
    fn uses(&self) -> &[PanelUse] {
        PanelUsage::uses(self)
    }

    fn record(&mut self, item: &str) {
        if let Err(error) = PanelUsage::record(self, item) {
            tracing::warn!(%error, "面板使用记录写不进去");
        }
    }
}

/// 格式不对的行跳过。
fn parse(text: &str) -> Vec<PanelUse> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let item = fields.next().filter(|item| !item.is_empty())?;
            let count = fields.next()?.parse().ok()?;
            let last = fields.next()?.parse().ok()?;
            Some(PanelUse {
                item: item.to_owned(),
                count,
                last,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_reads_back() {
        let dir = std::env::temp_dir().join(format!("qj-panel-usage-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut usage = PanelUsage::open(&dir);
        usage.record_at("★", 10).unwrap();
        usage.record_at("★", 20).unwrap();
        usage.record_at("😂", 15).unwrap();
        let reopened = PanelUsage::open(&dir);
        assert_eq!(reopened.uses().len(), 2);
        assert_eq!(reopened.uses()[0].count, 2);
        assert_eq!(reopened.uses()[0].last, 20);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn skips_bad_lines() {
        assert_eq!(parse("★\t2\t5\nbroken\n\t1\t1\n√\tx\t1\n").len(), 1);
    }
}
