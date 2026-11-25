//! 侧栏搜索：搜各页设置行、分组标题、勾选项的标题与说明全文，点结果跳到那页并把那一行醒目标一下。
//! 说明里命中的结果带一段摘录（命中处前后几个字），列表里写在标题下面。
//! 索引是离屏把每页建一遍收来的（[`crate::view::index`]），开始搜时建一次，清空搜索时扔掉。

mod entry;
mod hit;

pub use entry::Entry;
pub use hit::Hit;

use std::time::{Duration, Instant};

use bamboo_slip::{Key, TextEdit};

/// 醒目标出留多久再淡出。
pub const SPOTLIGHT: Duration = Duration::from_millis(1200);

/// 最多列几条结果。
const LIMIT: usize = 20;

/// 摘录里命中处前、后各留几个字。
const BEFORE: usize = 8;
const AFTER: usize = 12;

#[derive(Default)]
pub struct Search {
    pub edit: TextEdit,

    /// 这次搜索用的索引；没在搜时为 `None`。
    pub index: Option<Vec<Entry>>,

    /// 按当前输入筛出来的结果。
    pub hits: Vec<Hit>,

    /// 键盘 ↑ ↓ 选到的那条；没按过是 `None`，回车跳第一条。
    pub hot: Option<usize>,

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

    /// 按输入重筛：标题里有的排最前，其次控件上的字与同义词，再次说明（带摘录）；同一页同一标题只留一条。
    pub fn refresh(&mut self) {
        self.hot = None;
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
        let mut hits: Vec<Hit> = Vec::new();
        for (rank, entry) in ranked {
            let seen = hits
                .iter()
                .any(|hit| hit.page == entry.page && hit.title == entry.title);
            if !seen {
                let snippet = (rank == 2)
                    .then(|| {
                        entry
                            .detail
                            .as_deref()
                            .and_then(|text| excerpt(text, &query))
                    })
                    .flatten();
                hits.push(Hit {
                    page: entry.page,
                    key: entry.key,
                    title: entry.title.clone(),
                    snippet,
                    region: entry.region,
                });
            }
            if hits.len() == LIMIT {
                break;
            }
        }
        self.hits = hits;
    }

    /// ↑ ↓ 在结果里移一条，到头停住。
    pub fn step(&mut self, down: bool) {
        let Some(last) = self.hits.len().checked_sub(1) else {
            return;
        };
        self.hot = Some(match (self.hot, down) {
            (None, _) => 0,
            (Some(hot), true) => (hot + 1).min(last),
            (Some(hot), false) => hot.saturating_sub(1),
        });
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

/// `text` 里 `query`（已转小写）命中处前后几个字，两头截掉的补「…」。
fn excerpt(text: &str, query: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let lower: Vec<char> = chars
        .iter()
        .map(|c| c.to_lowercase().next().unwrap_or(*c))
        .collect();
    let needle: Vec<char> = query.chars().collect();
    let at = lower
        .windows(needle.len().max(1))
        .position(|window| window == needle.as_slice())?;
    let from = at.saturating_sub(BEFORE);
    let to = (at + needle.len() + AFTER).min(chars.len());
    let mut out = String::new();
    if from > 0 {
        out.push('…');
    }
    out.extend(&chars[from..to]);
    if to < chars.len() {
        out.push('…');
    }
    Some(out)
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
            region: None,
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
        let snippets: Vec<Option<&str>> = search
            .hits
            .iter()
            .map(|hit| hit.snippet.as_deref())
            .collect();
        assert_eq!(snippets, [None, None, None, Some("打字时向云端要候选")]);
    }

    #[test]
    fn excerpts_keep_a_few_characters_around_the_match() {
        assert_eq!(
            excerpt("候选窗里每个词后面跟一个外语译词，学着用", "外语").as_deref(),
            Some("…每个词后面跟一个外语译词，学着用")
        );
        assert_eq!(
            excerpt("Shows the Chinese and English words", "english").as_deref(),
            Some("…ese and English words")
        );
        assert_eq!(excerpt("没有", "云端"), None);
    }

    #[test]
    fn arrow_keys_move_within_the_results() {
        let mut search = Search {
            hits: vec![
                Hit {
                    page: PageId::Input,
                    key: None,
                    title: "甲".to_owned(),
                    snippet: None,
                    region: None,
                };
                2
            ],
            ..Search::default()
        };
        search.step(false);
        assert_eq!(search.hot, Some(0));
        search.step(true);
        search.step(true);
        assert_eq!(search.hot, Some(1));
        search.step(false);
        search.step(false);
        assert_eq!(search.hot, Some(0));
    }
}
