//! 「词库」页：随包领域词库（两列勾选），导入的拼音与五笔词库（开关、移除、导入），从词库仓库下载的词库。基础词库一直启用，不列。
//! 下载的词库装进用户词库目录，和导入的一样列在「导入的词库」里开关、移除；可下载的清单在设置窗口打开时取（见 `jobs::dict_index`）。

use std::time::SystemTime;

use bamboo_slip::taffy::{
    AlignItems, FlexDirection, JustifyContent, LengthPercentage, NodeId, Rect, Style,
};
use bamboo_slip::{Button, RenderError, SettingRow, Ui, Visual, group, paragraph, section_title};

use crate::i18n::{tr, tr_args};
use crate::jobs::dict_index::{DictEntry, Status};
use crate::jobs::download::{Item, Stage};
use crate::library::Kind;
use crate::message::Message;
use crate::view::Page;
use crate::view::parts::download::{megabytes, progress};
use crate::view::parts::lists::{domain_grid, entry_row, import_row, toggle_disabled};

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
    let domains = if library.domains.is_empty() {
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
    rows.push(import_row(
        ui,
        page,
        Kind::Dictionary,
        tr("dictionaries.import"),
        tr("dictionaries.import_info"),
    )?);
    out.push(section_title(
        ui,
        theme,
        tr("dictionaries.section_imported"),
    )?);
    out.push(group(ui, theme, &rows)?);

    out.push(section_title(
        ui,
        theme,
        tr("dictionaries.section_download"),
    )?);
    if let Some(("dicts_download", error)) = &page.settings.notice {
        out.push(paragraph(ui, theme, error, None)?);
    }
    out.extend(downloads(ui, page)?);
    Ok(out)
}

/// 可下载的词库：分组标题下先一行写清单是什么时候取的、这次取得怎么样，右边「刷新」；再是卡片，一本一行。
fn downloads(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let index = page.settings.dict_index.state();
    let mut out = Vec::new();
    let time = index.fetched_at.map(fetched_time);
    let loading = index.status == Status::Loading;
    let text = match (&index.status, &time) {
        (Status::Loading, Some(_)) => tr("dictionaries.list_refreshing").to_owned(),
        (Status::Loading | Status::Idle, None) => tr("dictionaries.list_loading").to_owned(),
        (Status::Failed(error), Some(time)) => tr_args(
            "dictionaries.list_stale",
            &[("error", error), ("time", time)],
        ),
        (Status::Failed(error), None) => tr_args("dictionaries.index_failed", &[("error", error)]),
        (Status::Fetched, _) if index.dicts.is_empty() => {
            tr("dictionaries.no_downloads").to_owned()
        }
        (Status::Fetched | Status::Idle, Some(time)) => {
            tr_args("dictionaries.list_source", &[("time", time)])
        }
        (Status::Fetched, None) => tr("dictionaries.no_downloads").to_owned(),
    };
    let note = paragraph(ui, theme, &text, None)?;
    let refresh = Button::new(tr("dictionaries.refresh"))
        .disabled(loading)
        .build(ui, theme, "dict-index", Message::FetchDictIndex)?;
    out.push(ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            justify_content: JustifyContent::SPACE_BETWEEN,
            gap: page.gap(12.0),
            padding: Rect {
                left: LengthPercentage::length(page.px(4.0)),
                right: LengthPercentage::length(page.px(4.0)),
                top: LengthPercentage::length(0.0),
                bottom: LengthPercentage::length(0.0),
            },
            ..Style::default()
        },
        Visual::Group,
        &[note, refresh],
    )?);
    if !index.dicts.is_empty() {
        let mut rows = Vec::new();
        for entry in &index.dicts {
            rows.extend(dict_row(ui, page, entry)?);
        }
        out.push(group(ui, theme, &rows)?);
    }
    Ok(out)
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

/// 一本可下载的词库：没装给「下载」，下着给进度与「取消」，装好了版本一样写「已安装」，版本不同给「更新」。
fn dict_row(
    ui: &mut Ui<Message>,
    page: &Page,
    entry: &DictEntry,
) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let settings = page.settings;
    let id = entry.id.as_str();
    let mut detail = format!("{} · {}", megabytes(entry.bytes), entry.license);
    let description = entry.description.trim_end_matches(['。', '.']);
    if !description.is_empty() {
        detail = format!("{description} · {detail}");
    }
    if !entry.official {
        detail = format!("{} · {detail}", tr("dictionaries.third_party"));
    }
    let installed = settings
        .library
        .dictionaries
        .iter()
        .find(|installed| installed.stem == entry.stem());
    let stage = settings.download.stage_of(&Item::Dict(entry.id.clone()));
    let mut rows = Vec::new();
    let mut row = SettingRow::new(&entry.name).detail(&detail);
    let download = Message::DownloadDict(entry.id.clone());
    match &stage {
        Some(Stage::Downloading { done, total }) => {
            let cancel = Button::new(tr("experimental.cancel")).build(
                ui,
                theme,
                ("dict-cancel", id),
                Message::CancelDownload,
            )?;
            rows.push(row.control(cancel).build(ui, theme)?);
            rows.push(progress(ui, page, *done, *total)?);
            return Ok(rows);
        }
        Some(Stage::Installing) => {
            let waiting = Button::new(tr("experimental.installing"))
                .disabled(true)
                .build(ui, theme, ("dict-installing", id), Message::CancelDownload)?;
            rows.push(row.control(waiting).build(ui, theme)?);
            return Ok(rows);
        }
        _ => {}
    }
    let busy = settings.download.busy();
    let button = if entry.needs_newer_app() {
        Button::new(tr("dictionaries.needs_newer_app")).disabled(true)
    } else if installed.is_some_and(|installed| installed.version == entry.version) {
        Button::new(tr("dictionaries.installed")).disabled(true)
    } else if installed.is_some() {
        Button::new(tr("dictionaries.update")).disabled(busy)
    } else if let Some(Stage::Failed(error)) = &stage {
        row = row.below(paragraph(
            ui,
            theme,
            &tr_args("experimental.download_failed", &[("error", &error)]),
            None,
        )?);
        Button::new(tr("experimental.retry")).disabled(busy)
    } else {
        Button::new(tr("experimental.download")).disabled(busy)
    };
    let button = button.build(ui, theme, ("dict-download", id), download)?;
    rows.push(row.control(button).build(ui, theme)?);
    Ok(rows)
}
