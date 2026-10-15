//! 侧栏搜索：搜各页设置行、分组标题、勾选项的标题与 ⓘ 全文，点结果跳到那页并把那一行醒目标一下。
//! 索引是离屏把每页建一遍收来的（[`crate::view::index`]），开始搜时建一次，清空搜索时扔掉。

mod entry;

pub use entry::Entry;

use std::time::{Duration, Instant};

use bamboo_slip::{Key, TextEdit};

/// 醒目标出留多久再淡出。
pub const SPOTLIGHT: Duration = Duration::from_millis(1200);

/// 最多列几条结果。
const LIMIT: usize = 30;

#[derive(Default)]
pub struct Search {
    pub edit: TextEdit,

    /// 这次搜索用的索引；没在搜时为 `None`。
    pub index: Option<Vec<Entry>>,

    /// 按当前输入筛出来的结果。
    pub hits: Vec<Entry>,

    /// 跳过去要醒目标出的元素与撤掉的时刻。
    pub spotlight: Option<(Key, Instant)>,
}

impl Search {
    pub fn query(&self) -> &str {
        self.edit.text().trim()
    }

    pub fn active(&self) -> bool {
        !self.query().is_empty()
    }

    /// 按输入重筛：标题里有的排最前，其次控件上的字与同义词，再次 ⓘ 说明；同一页同一标题只留一条。
    pub fn refresh(&mut self) {
        let query = self.query().to_lowercase();
        let Some(index) = self.index.as_ref().filter(|_| !query.is_empty()) else {
            self.hits.clear();
            return;
        };
        let has = |text: &str| text.to_lowercase().contains(&query);
        let rank = |entry: &Entry| {
            if has(&entry.title) {
                Some(0)
            } else if entry.keywords.iter().any(|word| has(word)) {
                Some(1)
            } else if entry.detail.as_deref().is_some_and(has) {
                Some(2)
            } else {
                None
            }
        };
        let mut ranked: Vec<(u8, &Entry)> = index
            .iter()
            .filter_map(|entry| rank(entry).map(|rank| (rank, entry)))
            .collect();
        // 稳定排序：同一档里保持页序
        ranked.sort_by_key(|(rank, _)| *rank);
        let mut hits: Vec<Entry> = Vec::new();
        for (_, entry) in ranked {
            let seen = hits
                .iter()
                .any(|hit| hit.page == entry.page && hit.title == entry.title);
            if !seen {
                hits.push(entry.clone());
            }
            if hits.len() == LIMIT {
                break;
            }
        }
        self.hits = hits;
    }

    pub fn clear(&mut self) {
        *self = Self {
            spotlight: self.spotlight,
            ..Self::default()
        };
    }

    /// 醒目标出到时间了就撤掉，返回是否撤了。
    pub fn expire(&mut self, now: Instant) -> bool {
        let expired = self.spotlight.is_some_and(|(_, until)| now >= until);
        if expired {
            self.spotlight = None;
        }
        expired
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page_id::PageId;

    fn entry(page: PageId, title: &str, detail: Option<&str>) -> Entry {
        Entry {
            page,
            key: None,
            title: title.to_owned(),
            detail: detail.map(str::to_owned),
            keywords: Vec::new(),
        }
    }

    #[test]
    fn titles_rank_before_control_words_before_details_and_duplicates_collapse() {
        let mut search = Search {
            index: Some(vec![
                entry(PageId::Cloud, "启用", Some("打字时向云端要候选")),
                entry(PageId::Input, "云端词", None),
                entry(PageId::Cloud, "云端词", None),
                entry(PageId::Cloud, "云端词", Some("重复登记")),
                Entry {
                    keywords: vec!["云端模型".to_owned()],
                    ..entry(PageId::Candidates, "联想", None)
                },
            ]),
            ..Search::default()
        };
        search.edit = TextEdit::new("云端".to_owned());
        search.refresh();
        let found: Vec<(PageId, &str)> = search
            .hits
            .iter()
            .map(|hit| (hit.page, hit.title.as_str()))
            .collect();
        assert_eq!(
            found,
            [
                (PageId::Input, "云端词"),
                (PageId::Cloud, "云端词"),
                (PageId::Candidates, "联想"),
                (PageId::Cloud, "启用")
            ]
        );
    }
}
