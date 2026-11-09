//! 「生词本」页的内容：读学习数据目录里的生词本，按页面的筛选列出，最近见到的在前；标认识、删除也在这里改文件。
//! 输入法那边的生词本落盘前会合并文件（Core 的 `VocabularyBook::refresh`），两边各改各的不会互相盖掉。

mod change;
mod filter;
mod page;
mod request;
mod word;

pub use change::VocabularyChange;
pub use filter::VocabularyFilter;
pub use page::VocabularyPage;
pub use request::VocabularyRequest;
pub use word::VocabularyWord;

use std::path::Path;

use qingjian_learning::VocabularyBook;
use qingjian_learning::figures::recent_start;
use qingjian_translate::LevelTable;

use crate::options::learning_language;
use filter::is_fresh;

/// 页面一次最多给这么多条：手机上翻不完，再多就该做分页了。
pub const PAGE_LIMIT: usize = 500;

/// 学习语言关着时没有生词本，返回空页。
pub fn vocabulary(data_dir: &Path, user_dir: &Path, request: &VocabularyRequest) -> VocabularyPage {
    let Some(language) = learning_language(&request.language) else {
        return VocabularyPage::default();
    };
    let mut book = VocabularyBook::open(user_dir.join("user-vocab.tsv"));
    if let Some(change) = &request.change {
        let changed = match change {
            VocabularyChange::Known { word, known } => book.set_known(language, word, *known),
            VocabularyChange::Remove { word } => book.remove(language, word),
        };
        if changed && let Err(error) = book.save() {
            tracing::warn!(%error, "生词本写不进去");
        }
    }
    let levels = LevelTable::from_path(data_dir.join(format!("levels-{}.tsv", language.code())))
        .inspect_err(|error| tracing::debug!(%error, "这门语言没有等级表"))
        .ok();
    if let Some(table) = &levels {
        book = book.with_levels(language, table.clone());
    }
    let today = jiff::Zoned::now().date();
    let recent_from = recent_start(today);
    let summary = book.summary_on(language, today);
    let level_of = |word: &str| {
        levels
            .as_ref()
            .and_then(|table| table.level(word))
            .unwrap_or_default()
            .to_owned()
    };
    let search = request.search.trim().to_lowercase();
    let mut entries: Vec<_> = book
        .words(language)
        .filter(|(_, entry)| request.filter.keeps(entry, recent_from))
        .filter(|(word, _)| request.level.is_empty() || level_of(word) == request.level)
        .filter(|(word, entry)| {
            search.is_empty()
                || word.to_lowercase().contains(&search)
                || entry.source.contains(&search)
                || entry.example.to_lowercase().contains(&search)
        })
        .collect();
    entries.sort_by(|(a_word, a), (b_word, b)| {
        b.last
            .cmp(&a.last)
            .then(b.committed.cmp(&a.committed))
            .then(a_word.cmp(b_word))
    });
    VocabularyPage {
        seen: summary.seen,
        familiar: summary.familiar,
        new_this_week: summary.new_this_week,
        levels: summary.levels.into_iter().map(|level| level.name).collect(),
        matched: entries.len() as u64,
        words: entries
            .into_iter()
            .take(PAGE_LIMIT)
            .map(|(word, entry)| VocabularyWord {
                word: word.to_owned(),
                source: entry.source.clone(),
                example: entry.example.clone(),
                seen: entry.seen,
                committed: entry.committed,
                used: entry.used,
                last: entry.last.to_string(),
                known: entry.known,
                fresh: is_fresh(entry),
                level: level_of(word),
            })
            .collect(),
    }
}

/// 请求读不懂按缺省（英语、全部）列。
pub(crate) fn vocabulary_json(data_dir: &Path, user_dir: &Path, request: &str) -> String {
    let request = serde_json::from_str(request).unwrap_or_else(|error| {
        tracing::warn!(%error, "生词本请求读不懂");
        VocabularyRequest {
            language: "en".to_owned(),
            filter: VocabularyFilter::All,
            level: String::new(),
            search: String::new(),
            change: None,
        }
    });
    serde_json::to_string(&vocabulary(data_dir, user_dir, &request)).expect("生词本总能序列化")
}

#[cfg(test)]
mod tests;
