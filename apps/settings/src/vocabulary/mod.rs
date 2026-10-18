//! 「生词本」页的数据：时间线（`vocab-log.tsv`，每次相遇一行）与词表（`user-vocab.tsv`，译词连同带它上屏的中文与例句）。
//! 进页时读；标认识、删除、清空当场写回，写之前先合并输入法那边新记的（见 [`VocabularyBook::refresh`]）。
//! 老记录没有中文的英语词，用随包的英→中释义表补一个（只显示、导出，不写回）。

mod export;
mod filter;
mod format;
mod sort;
mod timeline;
mod view;

pub use filter::Filter;
pub use format::Format;
pub use sort::Sort;
pub use view::View;

use std::collections::BTreeSet;
use std::path::PathBuf;

use jiff::civil::Date;
use qingjian_core::{Language, Translator, VocabularySummary};
use qingjian_learning::{LoggedEncounter, UsageStats, VocabularyBook, VocabularyEntry};
use qingjian_platform::GeneralConfig;
use qingjian_translate::Glossary;

use crate::stats::{language_name, week_start};
use crate::system::paths;

/// 一页列几个词。
pub const PAGE_SIZE: usize = 50;

pub struct Vocabulary {
    book: VocabularyBook,

    pub view: View,

    /// 时间线正在看的那天。
    pub day: Date,

    /// 正在看的语言的全部相遇，按时间先后。
    encounters: Vec<LoggedEncounter>,

    /// 打过字的日子（输入统计），算连续输入几天。
    typed_days: BTreeSet<Date>,

    /// 英→中释义表，给没有中文的老记录补一个；没装或不是英语为 `None`。
    glossary: Option<Glossary>,

    /// 正在看的语言：进页时是当前的学习语言，可以切到别的有记录的语言。
    pub language: Language,

    /// 能切的语言：当前学习语言在前，后面是本子里有记录的别的语言。
    pub languages: Vec<Language>,

    pub filter: Filter,

    pub sort: Sort,

    /// 第几页，从 0 起。
    pub page: usize,

    /// 「清空」点了一下，等再点一次。
    pub confirm_clear: bool,

    today: Date,
}

impl Vocabulary {
    /// 学习语言关着为 `None`。
    pub fn load(general: &GeneralConfig) -> Option<Self> {
        let mut vocabulary = Self::with_book(VocabularyBook::open(path()), language(general)?);
        let dir = paths::user_dir().unwrap_or_default();
        vocabulary.typed_days = UsageStats::open(&dir)
            .days()
            .into_iter()
            .filter(|(_, usage)| usage.hanzi > 0)
            .map(|(date, _)| date)
            .collect();
        vocabulary.glossary = english_glossary();
        vocabulary.reload();
        Some(vocabulary)
    }

    /// 还没读文件时（搜索索引离屏建页）用的空本子。
    pub fn empty(general: &GeneralConfig) -> Option<Self> {
        Some(Self::with_book(
            VocabularyBook::default(),
            language(general)?,
        ))
    }

    pub fn with_book(book: VocabularyBook, language: Language) -> Self {
        let mut languages = vec![language];
        languages.extend(book.languages().into_iter().filter(|l| *l != language));
        let today = jiff::Zoned::now().date();
        Self {
            book,
            view: View::default(),
            day: today,
            encounters: Vec::new(),
            typed_days: BTreeSet::new(),
            glossary: None,
            language,
            languages,
            filter: Filter::default(),
            sort: Sort::default(),
            page: 0,
            confirm_clear: false,
            today,
        }
    }

    /// 换一门语言看：时间线重读，回到今天。
    pub fn set_language(&mut self, language: Language) {
        self.language = language;
        self.day = self.today;
        self.page = 0;
        self.reload();
    }

    fn reload(&mut self) {
        self.encounters = self.book.encounters(self.language);
    }

    /// 带它上屏的中文；老记录没有时英语查释义表补一个。
    pub fn source<'a>(&self, word: &str, entry: &'a VocabularyEntry) -> std::borrow::Cow<'a, str> {
        if !entry.source.is_empty() {
            return entry.source.as_str().into();
        }
        self.glossary
            .as_ref()
            .filter(|_| self.language == Language::English)
            .and_then(|glossary| glossary.translate(word))
            .map(|translation| {
                translation
                    .senses()
                    .iter()
                    .map(|sense| sense.text.as_str())
                    .collect::<Vec<_>>()
                    .join("；")
            })
            .unwrap_or_default()
            .into()
    }

    pub fn language_name(&self) -> &'static str {
        language_name(self.language)
    }

    pub fn summary(&self) -> VocabularySummary {
        self.book.summary_on(self.language, self.today)
    }

    /// 按筛选、搜索、排序排好的全部词；搜索不分大小写，译词、中文、例句里有就算。
    pub fn matching(&self, query: &str) -> Vec<(&str, &VocabularyEntry)> {
        let query = query.trim().to_lowercase();
        let week = week_start(self.today);
        let mut words: Vec<_> = self
            .book
            .words(self.language)
            .filter(|(_, entry)| self.filter.keeps(entry, week))
            .filter(|(word, entry)| {
                query.is_empty()
                    || word.to_lowercase().contains(&query)
                    || self.source(word, entry).contains(&query)
                    || entry.example.to_lowercase().contains(&query)
            })
            .collect();
        words.sort_by(|a, b| self.sort.compare(*a, *b));
        words
    }

    pub fn pages(count: usize) -> usize {
        count.div_ceil(PAGE_SIZE).max(1)
    }

    pub fn set_known(&mut self, word: &str, known: bool) -> Result<(), String> {
        self.book.refresh();
        self.book.set_known(self.language, word, known);
        self.save()
    }

    pub fn remove(&mut self, word: &str) -> Result<(), String> {
        self.book.refresh();
        self.book.remove(self.language, word);
        self.save()
    }

    pub fn clear(&mut self) -> Result<(), String> {
        self.book.refresh();
        self.book.clear(self.language);
        self.page = 0;
        let saved = self.save();
        self.reload();
        saved
    }

    /// 弹出存文件的窗口，把当前筛出来的词写进去；没选位置返回 `Ok(None)`。
    pub fn export(&self, format: Format, query: &str) -> Result<Option<PathBuf>, String> {
        let words = self.matching(query);
        if words.is_empty() {
            return Err("没有可导出的词".to_owned());
        }
        let name = format!("青简生词本-{}.{}", self.today, format.extension());
        let Some(target) = rfd::FileDialog::new()
            .set_title("导出生词本")
            .set_file_name(&name)
            .add_filter(format.label(), &[format.extension()])
            .save_file()
        else {
            return Ok(None);
        };
        // 补过中文的老记录导出时也带上
        let filled: Vec<(&str, VocabularyEntry)> = words
            .iter()
            .map(|(word, entry)| {
                let mut entry = (*entry).clone();
                entry.source = self.source(word, &entry).into_owned();
                (*word, entry)
            })
            .collect();
        let words: Vec<(&str, &VocabularyEntry)> =
            filled.iter().map(|(word, entry)| (*word, entry)).collect();
        let text = export::render(format, self.language_name(), &words);
        std::fs::write(&target, text).map_err(|error| format!("写不进去：{error}"))?;
        Ok(Some(target))
    }

    fn save(&mut self) -> Result<(), String> {
        self.book.save().map_err(|error| format!("没存上：{error}"))
    }
}

fn language(general: &GeneralConfig) -> Option<Language> {
    (!general.learning_language_off()).then(|| {
        general
            .learning_language
            .parse::<Language>()
            .unwrap_or(Language::English)
    })
}

/// 英→中释义表：macOS 在 `Qingjian.app/Contents/Resources/`，其他平台在随包根的 `data/generated/`。
fn english_glossary() -> Option<Glossary> {
    let path = paths::mac_resources()
        .into_iter()
        .map(|dir| dir.join("glossary-zh.qj"))
        .find(|path| path.is_file())
        .or_else(|| paths::bundled("data/generated/glossary-zh.qj"))?;
    Glossary::from_path(Language::Chinese, &path)
        .inspect_err(|error| tracing::warn!(%error, "英→中释义表读不了"))
        .ok()
}

fn path() -> PathBuf {
    paths::user_dir().unwrap_or_default().join("user-vocab.tsv")
}
