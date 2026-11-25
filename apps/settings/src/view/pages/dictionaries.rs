//! 「词库」页：随包领域词库（两列勾选），我的词库（导入的拼音与五笔词库、下载的词库：开关、移除、导入），从词库仓库下载的词库。基础词库一直启用，不列。
//! 下载的词库装进用户词库目录，和导入的一样列在「我的词库」里开关、移除；下载的释义表装进用户目录 `glossaries/`，输入法换上就用。
//! 可下载的清单在设置窗口打开时取（见 `jobs::dict_index`）。

use std::time::SystemTime;

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{
    Button, DownloadRow, FieldValue, Link, Look, RenderError, Segmented, Ui, field_points, fields,
    group, paragraph, section_title, toolbar,
};

use qingjian_core::Language;

use crate::i18n::{tr, tr_args};
use crate::jobs::dict_index::{DictEntry, IndexState, Shelf, Status};
use crate::jobs::download::Item;
use crate::library::Kind;
use crate::message::Message;
use crate::stats::language_name;
use crate::view::Page;
use crate::view::parts::download::{DownloadStatus, labels, megabytes};
use crate::view::parts::lists::{
    domain_grid, entry_row, import_row, license_label, toggle_disabled, with_actions,
};

pub fn dictionaries(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let library = &page.settings.library;
    let config = &page.settings.config.dictionaries;
    let mut out = Vec::new();

    out.push(section_title(
        ui,
        theme,
        tr("dictionaries.section_bundled"),
    )?);
    let domains = if library.base.is_none() && library.domains.is_empty() {
        paragraph(ui, theme, tr("dictionaries.no_bundled"), None)?
    } else {
        domain_grid(ui, page)?
    };
    out.push(group(ui, theme, &[domains])?);

    let mut rows = Vec::new();
    let imported = library
        .dictionaries
        .iter()
        .map(|entry| (Kind::Dictionary, entry))
        .chain(
            library
                .code_words
                .iter()
                .map(|entry| (Kind::CodeWords, entry)),
        );
    for (kind, entry) in imported {
        let toggle = toggle_disabled(kind, &config.disabled, &entry.stem);
        rows.push(entry_row(
            ui,
            page,
            kind,
            entry,
            Some((config.is_enabled(&entry.stem), toggle)),
            true,
            None,
        )?);
    }
    let import = import_row(
        ui,
        page,
        Kind::Dictionary,
        tr("dictionaries.import"),
        tr("dictionaries.import_info"),
    )?;
    let rows = with_actions(rows, vec![import]);
    out.push(section_title(
        ui,
        theme,
        tr("dictionaries.section_imported"),
    )?);
    out.push(group(ui, theme, &rows)?);

    if theme.look == Look::Plain {
        out.push(section_title(
            ui,
            theme,
            tr("dictionaries.section_download"),
        )?);
        out.extend(notice(ui, page)?);
        out.extend(downloads(ui, page)?);
    } else {
        out.extend(both_shelves(ui, page)?);
    }
    Ok(out)
}

/// 下载、删除出错时的一句。
fn notice(ui: &mut Ui<Message>, page: &Page) -> Result<Option<NodeId>, RenderError> {
    match &page.settings.notice {
        Some(("dicts_download", error)) => Ok(Some(paragraph(ui, &page.theme, error, None)?)),
        _ => Ok(None),
    }
}

/// 两种原生外观：「可下载的词库」「可下载的释义表」各一节，不用分段切换。
/// 词库一节先一行小字写列表的来历、右边「刷新」；释义表一节标出当前译文语言那一张。
fn both_shelves(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let index = page.settings.dict_index.state();
    let mut out = vec![section_title(
        ui,
        theme,
        tr("dictionaries.section_download_dicts"),
    )?];
    out.extend(notice(ui, page)?);
    let dicts = shelf_entries(&index, Shelf::Dictionaries);
    let text = status_text(&index, dicts.is_empty(), tr("dictionaries.no_downloads"));
    let note = paragraph(ui, theme, &text, None)?;
    let refresh = Button::new(tr("dictionaries.refresh"))
        .disabled(index.status == Status::Loading)
        .build(ui, theme, "dict-index", Message::FetchDictIndex)?;
    out.push(toolbar(ui, theme, &[note, refresh])?);
    out.extend(shelf_groups(ui, page, Shelf::Dictionaries, dicts)?);

    out.push(section_title(
        ui,
        theme,
        tr("dictionaries.section_download_glossaries"),
    )?);
    let glossaries = shelf_entries(&index, Shelf::Glossaries);
    if glossaries.is_empty() {
        if index.status == Status::Fetched {
            let empty = paragraph(ui, theme, tr("dictionaries.no_glossaries"), None)?;
            out.push(toolbar(ui, theme, &[empty])?);
        }
    } else {
        out.extend(shelf_groups(ui, page, Shelf::Glossaries, glossaries)?);
    }
    Ok(out)
}

/// 清单里属于这一类的。
fn shelf_entries(index: &IndexState, shelf: Shelf) -> Vec<&DictEntry> {
    index
        .dicts
        .iter()
        .filter(|entry| shelf.holds(entry))
        .collect()
}

/// 列表是什么时候取的、这次取得怎么样；取到了但一本也没有时写 `empty`。
fn status_text(index: &IndexState, none: bool, empty: &str) -> String {
    let time = index.fetched_at.map(fetched_time);
    match (&index.status, &time) {
        (Status::Loading, Some(_)) => tr("dictionaries.list_refreshing").to_owned(),
        (Status::Loading | Status::Idle, None) => tr("dictionaries.list_loading").to_owned(),
        (Status::Failed(error), Some(time)) => tr_args(
            "dictionaries.list_stale",
            &[("error", error), ("time", time)],
        ),
        (Status::Failed(error), None) => tr_args("dictionaries.index_failed", &[("error", error)]),
        (Status::Fetched, _) if none => empty.to_owned(),
        (Status::Fetched | Status::Idle, Some(time)) => {
            tr_args("dictionaries.list_source", &[("time", time)])
        }
        (Status::Fetched, None) => empty.to_owned(),
    }
}

/// 一类的卡片：词库按语言分组（不止一种语言时每组上面写语言名），释义表一张一门语言、名字里就有，不分组。
fn shelf_groups(
    ui: &mut Ui<Message>,
    page: &Page,
    shelf: Shelf,
    shown: Vec<&DictEntry>,
) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let shelves = match shelf {
        Shelf::Dictionaries => by_language(&shown),
        Shelf::Glossaries if shown.is_empty() => Vec::new(),
        Shelf::Glossaries => vec![(String::new(), shown)],
    };
    let titled = shelves.len() > 1;
    let mut out = Vec::new();
    for (language, entries) in shelves {
        if titled {
            let title = paragraph(ui, theme, &language_label(&language), None)?;
            out.push(toolbar(ui, theme, &[title])?);
        }
        let mut rows = Vec::new();
        for entry in entries {
            rows.push(dict_row(ui, page, entry)?);
        }
        out.push(group(ui, theme, &rows)?);
    }
    Ok(out)
}

/// 竹青：分组标题下先一行「词库 ｜ 释义表」与「刷新」，再一行小字写列表是什么时候取的、这次取得怎么样；
/// 再是选着那一类的卡片，一本一行。
fn downloads(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let settings = page.settings;
    let index = settings.dict_index.state();
    let shelf = settings.dict_shelf;
    let shown = shelf_entries(&index, shelf);
    let empty = match shelf {
        Shelf::Dictionaries => tr("dictionaries.no_downloads"),
        Shelf::Glossaries => tr("dictionaries.no_glossaries"),
    };
    let text = status_text(&index, shown.is_empty(), empty);
    let labels = [
        tr("dictionaries.shelf_dicts"),
        tr("dictionaries.shelf_glossaries"),
    ];
    let selected = Shelf::ALL
        .iter()
        .position(|item| *item == shelf)
        .unwrap_or(0);
    let tabs = Segmented::new(&labels, selected).build(ui, theme, "dict-shelf", |index| {
        Message::DictShelf(Shelf::ALL[index])
    })?;
    let refresh = Button::new(tr("dictionaries.refresh"))
        .disabled(index.status == Status::Loading)
        .build(ui, theme, "dict-index", Message::FetchDictIndex)?;
    let mut out = vec![toolbar(ui, theme, &[tabs, refresh])?];
    let note = paragraph(ui, theme, &text, None)?;
    out.push(toolbar(ui, theme, &[note])?);
    out.extend(shelf_groups(ui, page, shelf, shown)?);
    Ok(out)
}

/// 按语言分组，组的先后照清单里第一次出现的顺序；只有一种语言时调用方不写组名。
fn by_language<'a>(entries: &[&'a DictEntry]) -> Vec<(String, Vec<&'a DictEntry>)> {
    let mut groups: Vec<(String, Vec<&'a DictEntry>)> = Vec::new();
    for entry in entries {
        let language = entry.language.clone().unwrap_or_default();
        match groups.iter_mut().find(|(code, _)| *code == language) {
            Some((_, list)) => list.push(entry),
            None => groups.push((language, vec![entry])),
        }
    }
    groups
}

/// 组名：认得的语言写名字，不认得的照写代码。
fn language_label(code: &str) -> String {
    code.parse::<Language>().map_or_else(
        |_| code.to_owned(),
        |language| language_name(language).to_owned(),
    )
}

/// 清单取到的时间，按本机时区写到分钟。
fn fetched_time(at: SystemTime) -> String {
    jiff::Timestamp::try_from(at)
        .map(|stamp| {
            stamp
                .to_zoned(jiff::tz::TimeZone::system())
                .strftime("%Y-%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_default()
}

/// 一本可下载的词库或释义表：一行下载项（见 [`DownloadRow`]），说明下面写作者（有主页的能点）、大小、许可。
fn dict_row(ui: &mut Ui<Message>, page: &Page, entry: &DictEntry) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    let settings = page.settings;
    let id = entry.id.as_str();
    let library = &settings.library;
    let stem = entry.stem();
    let outdated = if entry.glossary_language().is_some() {
        &library.glossaries
    } else {
        &library.dictionaries
    }
    .iter()
    .find(|installed| installed.stem == stem)
    .map(|installed| installed.version != entry.version);
    let status = DownloadStatus::of(page, &Item::Dict(entry.id.clone()));
    let native = theme.look != Look::Plain;
    let current = entry.glossary_language().is_some_and(|language| {
        native && language.code() == settings.config.general.learning_language
    });
    DownloadRow::new(&entry.name, status.state(outdated), labels())
        .detail(entry.description.trim_end_matches(['。', '.']))
        .badge((!entry.official).then(|| tr("dictionaries.third_party")))
        .highlight(current.then(|| tr("dictionaries.current_language")))
        .mark_installed(native)
        .caption(|ui| caption(ui, page, entry))
        .blocked(entry.needs_newer_app())
        .armed(settings.dict_remove_armed.as_deref() == Some(id))
        .build(
            ui,
            theme,
            ("dict", id),
            (
                Message::DownloadDict(entry.id.clone()),
                Message::CancelDownload(Item::Dict(entry.id.clone())),
                Message::RemoveDownload(entry.id.clone()),
            ),
        )
}

/// 作者（有主页的能点）、大小、许可。
fn caption(ui: &mut Ui<Message>, page: &Page, entry: &DictEntry) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    let size = megabytes(entry.bytes);
    let license = license_label(&entry.license);
    let mut items = Vec::new();
    if !entry.author.name.is_empty() {
        let author = match entry.author.link() {
            Some(url) => FieldValue::Node(
                Link::new(&entry.author.name)
                    .points(field_points(theme))
                    .build(
                        ui,
                        theme,
                        ("dict-author", entry.id.as_str()),
                        Message::OpenLink(url.to_owned()),
                    )?,
            ),
            None => FieldValue::Text(&entry.author.name),
        };
        items.push((tr("dictionaries.author"), author));
    }
    items.push((tr("dictionaries.size"), FieldValue::Text(&size)));
    items.push((tr("dictionaries.license"), FieldValue::Text(&license)));
    fields(ui, theme, &items)
}
