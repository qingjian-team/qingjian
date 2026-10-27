//! 生词本的「词表」视图：一词一行（译词、中文、例句、次数），可搜索、筛选、排序、翻页，每行可标认识、删除。

use bamboo_slip::taffy::{
    AlignItems, FlexDirection, JustifyContent, LengthPercentage, LengthPercentageAuto, NodeId,
    Rect, Size, Style,
};
use bamboo_slip::{
    Button, RenderError, Segmented, SettingRow, TextField, Ui, Visual, group, paragraph,
    section_title,
};
use qingjian_core::FRESH_UNTIL;
use qingjian_learning::VocabularyEntry;

use super::parts::{pager, row, tag, text};
use crate::message::{FieldId, Message};
use crate::stats::group_digits;
use crate::view::Page;
use crate::vocabulary::{Filter, PAGE_SIZE, Sort, Vocabulary};

/// 搜索框的宽度（点）。
const SEARCH_WIDTH: f32 = 240.0;

pub fn words(
    ui: &mut Ui<Message>,
    page: &Page,
    book: &Vocabulary,
) -> Result<Vec<NodeId>, RenderError> {
    let query = page
        .settings
        .fields
        .get(&FieldId::VocabularySearch)
        .map(|edit| edit.text().to_owned())
        .unwrap_or_default();
    let words = book.matching(&query);
    let theme = &page.theme;
    let mut out = vec![paragraph(ui, theme, &summary_line(book), None)?];

    let filter = Segmented::new(
        &Filter::ALL.map(Filter::label),
        Filter::ALL
            .iter()
            .position(|f| *f == book.filter)
            .unwrap_or(0),
    )
    .build(ui, theme, "vocabulary-filter", |index| {
        Message::VocabularyFilter(Filter::ALL[index])
    })?;
    let sort = Segmented::new(
        &Sort::ALL.map(Sort::label),
        Sort::ALL.iter().position(|s| *s == book.sort).unwrap_or(0),
    )
    .build(ui, theme, "vocabulary-sort", |index| {
        Message::VocabularySort(Sort::ALL[index])
    })?;
    let mut rows = vec![
        SettingRow::new("筛选").control(filter).build(ui, theme)?,
        SettingRow::new("排序").control(sort).build(ui, theme)?,
    ];
    if let Some(edit) = page.settings.fields.get(&FieldId::VocabularySearch) {
        let id = FieldId::VocabularySearch;
        let search = TextField::new(edit)
            .placeholder("译词、中文或例句")
            .width(SEARCH_WIDTH)
            .build(
                ui,
                theme,
                ("field", id),
                move |edit| Message::Edit(id, edit),
                None,
                Some(Message::Cancel(id)),
            )?;
        rows.push(SettingRow::new("搜索").control(search).build(ui, theme)?);
    }
    out.push(group(ui, theme, &rows)?);

    let pages = Vocabulary::pages(words.len());
    let current = book.page.min(pages - 1);
    out.push(section_title(
        ui,
        theme,
        &format!("{} 个词", group_digits(words.len() as u64)),
    )?);
    let mut rows = Vec::new();
    for (word, entry) in words.iter().skip(current * PAGE_SIZE).take(PAGE_SIZE) {
        rows.push(word_row(ui, page, book, word, entry)?);
    }
    if rows.is_empty() {
        let empty = if query.trim().is_empty() && book.filter == Filter::All {
            "还没有记下译词：打中文时候选右侧的译词，上屏后就记在这里"
        } else {
            "没有符合的词"
        };
        rows.push(SettingRow::new(empty).build(ui, theme)?);
    }
    if pages > 1 {
        rows.push(pager(ui, page, current, pages)?);
    }
    out.push(group(ui, theme, &rows)?);

    Ok(out)
}

/// 「英语 · 生词 40 个，认识了 12 个，近 7 天新见 8 个」
fn summary_line(book: &Vocabulary) -> String {
    let summary = book.summary();
    if summary.seen == 0 {
        return format!("{}：还没有记录", book.language_name());
    }
    format!(
        "{}：共 {} 个词，生词 {} 个，近 7 天新见 {} 个",
        book.language_name(),
        group_digits(summary.seen),
        group_digits(summary.seen - summary.familiar),
        group_digits(summary.new_this_week),
    )
}

/// 一个词：左边译词、中文、例句、次数，右边「认识了」与「删除」。
fn word_row(
    ui: &mut Ui<Message>,
    page: &Page,
    book: &Vocabulary,
    word: &str,
    entry: &VocabularyEntry,
) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    let colors = &theme.colors;
    let mut title = vec![text(ui, page, word, 18.0, 700, colors.text)?];
    if entry.known {
        title.push(tag(ui, page, "认识了", false)?);
    } else if entry.seen < FRESH_UNTIL {
        title.push(tag(ui, page, "生词", true)?);
    }
    let mut lines = vec![row(ui, page, 10.0, &title)?];
    let source = book.source(word, entry);
    if !source.is_empty() {
        lines.push(text(ui, page, &source, theme.font.size, 400, colors.text)?);
    }
    if !entry.example.is_empty() {
        lines.push(paragraph(
            ui,
            theme,
            &format!("例句　{}", entry.example),
            None,
        )?);
    }
    lines.push(text(
        ui,
        page,
        &counts_line(entry),
        theme.small,
        400,
        colors.secondary,
    )?);
    let left = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            gap: page.gap(6.0),
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_size: Size {
                width: LengthPercentageAuto::length(0.0),
                height: LengthPercentageAuto::auto(),
            },
            ..Style::default()
        },
        Visual::Group,
        &lines,
    )?;
    let known = Button::new(if entry.known {
        "不认识了"
    } else {
        "认识了"
    })
    .build(
        ui,
        theme,
        ("vocabulary-known", word),
        Message::VocabularyKnown(word.to_owned(), !entry.known),
    )?;
    let remove = Button::new("删除").build(
        ui,
        theme,
        ("vocabulary-remove", word),
        Message::VocabularyRemove(word.to_owned()),
    )?;
    let right = row(ui, page, 8.0, &[known, remove])?;
    let space = LengthPercentage::length(page.px(12.0));
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            justify_content: JustifyContent::SPACE_BETWEEN,
            gap: page.gap(16.0),
            padding: Rect {
                top: space,
                bottom: space,
                left: LengthPercentage::length(0.0),
                right: LengthPercentage::length(0.0),
            },
            ..Style::default()
        },
        Visual::Group,
        &[left, right],
    )
}

/// 「看到 5 次 · 上屏 2 次 · 直接打出 1 次 · 最近 10月16日」
fn counts_line(entry: &VocabularyEntry) -> String {
    let mut parts = vec![
        format!("看到 {} 次", entry.seen),
        format!("上屏 {} 次", entry.committed),
    ];
    if entry.used > 0 {
        parts.push(format!("直接打出 {} 次", entry.used));
    }
    parts.push(format!(
        "最近 {}月{}日",
        entry.last.month(),
        entry.last.day()
    ));
    parts.join(" · ")
}
