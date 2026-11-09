//! 生词本的「时间线」视图：哪天（可往前后翻）、大字的相遇次数、当天第一次见到几个生词与连续输入几天、
//! 按小时的记忆色带（悬停看每个钟点几个词），下面一条条相遇（说法大字、中文在下、右边时刻，单个词再带一句例句）。

use bamboo_slip::taffy::{
    AlignItems, Dimension, FlexDirection, LengthPercentage, LengthPercentageAuto, NodeId, Rect,
    Size, Style,
};
use bamboo_slip::{
    BarChart, BoxPaint, Button, Datum, Fill, RenderError, SettingRow, Ui, Visual, group, paragraph,
    section_title,
};
use jiff::civil::Date;
use qingjian_learning::LoggedEncounter;

use super::parts::{pager, row, text, wrapped_text};
use crate::i18n::{tr, tr_args};
use crate::message::Message;
use crate::stats::group_digits;
use crate::view::Page;
use crate::vocabulary::{PAGE_SIZE, Vocabulary};

/// 色带的高（点）。
const BAND_HEIGHT: f32 = 72.0;

/// 卡片左右的内边距（点），见 `bamboo_slip::group`。
const CARD_PADDING: f32 = 16.0;

pub fn timeline(
    ui: &mut Ui<Message>,
    page: &Page,
    book: &Vocabulary,
) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let colors = &theme.colors;
    let encounters = book.day_encounters();
    let mut out = vec![day_switch(ui, page, book)?];

    let count = text(
        ui,
        page,
        &group_digits(encounters.len() as u64),
        56.0,
        800,
        colors.text,
    )?;
    let unit = text(ui, page, tr("timeline.met"), 20.0, 500, colors.secondary)?;
    out.push(row(ui, page, 10.0, &[count, unit])?);
    let day = if book.is_today() {
        tr("timeline.today")
    } else {
        tr("timeline.this_day")
    };
    let lines = tr_args(
        "timeline.first_seen",
        &[
            ("n", &group_digits(book.first_seen() as u64)),
            ("day", &day),
            ("streak", &book.streak()),
        ],
    );
    out.push(paragraph(ui, theme, &lines, Some(colors.text))?);
    let now = book.is_today().then(|| jiff::Zoned::now().hour() as usize);
    out.push(band(ui, page, &book.hours(), day, now)?);

    let title = if book.is_today() {
        tr("timeline.today").to_owned()
    } else {
        date_text(book.day)
    };
    out.push(section_title(
        ui,
        theme,
        &tr_args(
            "timeline.day_title",
            &[
                ("title", &title),
                ("n", &group_digits(encounters.len() as u64)),
            ],
        ),
    )?);
    let pages = Vocabulary::pages(encounters.len());
    let current = book.page.min(pages - 1);
    let mut rows = Vec::new();
    for encounter in encounters.iter().skip(current * PAGE_SIZE).take(PAGE_SIZE) {
        rows.push(item(ui, page, book, encounter)?);
    }
    if rows.is_empty() {
        rows.push(
            SettingRow::new(tr("timeline.none_today"))
                .info(tr("timeline.none_today_info"))
                .build(ui, theme)?,
        );
    }
    if pages > 1 {
        rows.push(pager(ui, page, current, pages)?);
    }
    out.push(group(ui, theme, &rows)?);
    Ok(out)
}

/// 「‹ 2026年10月16日 ›」：前后跳到有记录的日子。
fn day_switch(ui: &mut Ui<Message>, page: &Page, book: &Vocabulary) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    let (previous, next) = book.neighbors();
    let back = Button::new("‹").disabled(previous.is_none()).build(
        ui,
        theme,
        "vocabulary-day-previous",
        Message::VocabularyDay(previous.unwrap_or(book.day)),
    )?;
    let label = text(
        ui,
        page,
        &date_text(book.day),
        theme.font.size,
        500,
        theme.colors.secondary,
    )?;
    let forward = Button::new("›").disabled(next.is_none()).build(
        ui,
        theme,
        "vocabulary-day-next",
        Message::VocabularyDay(next.unwrap_or(book.day)),
    )?;
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            gap: page.gap(10.0),
            ..Style::default()
        },
        Visual::Group,
        &[back, label, forward],
    )
}

/// 记忆色带：一个钟点一根，跟统计页的柱状图一样，悬停浮出那个钟点遇到几个词；今天的话现在这个钟点高亮。
fn band(
    ui: &mut Ui<Message>,
    page: &Page,
    hours: &[usize; 24],
    day: &str,
    now: Option<usize>,
) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    let max = hours.iter().copied().max().unwrap_or(0).max(1);
    let bars = hours
        .iter()
        .enumerate()
        .map(|(hour, count)| {
            let tip = if *count == 0 {
                tr("timeline.none_hour").to_owned()
            } else {
                tr_args("timeline.hour_words", &[("n", count)])
            };
            Datum::new(
                *count as f32 / max as f32,
                format!("{hour}:00–{}:00 · {tip}", hour + 1),
            )
        })
        .collect();
    let labels = [0, 6, 12, 18]
        .into_iter()
        .map(|hour| (hour, format!("{hour}:00")))
        .collect();
    let width = page.content_width / page.scale - CARD_PADDING * 2.0;
    let chart = BarChart::new(bars, width, BAND_HEIGHT)
        .labels(labels)
        .end_label("24:00")
        .current(now)
        .build(ui, theme, "vocabulary-band")?;
    let title = text(
        ui,
        page,
        &tr_args("timeline.band", &[("day", &day)]),
        theme.small,
        500,
        theme.colors.secondary,
    )?;
    let space = LengthPercentage::length(page.px(14.0));
    let padded = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            gap: page.gap(10.0),
            padding: Rect {
                top: space,
                bottom: space,
                left: LengthPercentage::length(0.0),
                right: LengthPercentage::length(0.0),
            },
            ..Style::default()
        },
        Visual::Group,
        &[title, chart],
    )?;
    group(ui, theme, &[padded])
}

/// 一次相遇：左边一个圆点，说法大字（有读音跟在后面），中文在下，单个词再带例句；右边时刻。
fn item(
    ui: &mut Ui<Message>,
    page: &Page,
    book: &Vocabulary,
    encounter: &LoggedEncounter,
) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    let colors = &theme.colors;
    let dot = ui.scene().node(
        Style {
            size: Size {
                width: Dimension::length(page.px(10.0)),
                height: Dimension::length(page.px(10.0)),
            },
            flex_shrink: 0.0,
            margin: Rect {
                top: LengthPercentageAuto::length(page.px(8.0)),
                ..Rect::zero()
            },
            ..Style::default()
        },
        Visual::Box(BoxPaint {
            fill: Some(Fill::Solid(colors.line.lerp(colors.accent, 0.4))),
            border: None,
            radius: page.px(5.0),
        }),
        &[],
    )?;
    let mut title = vec![wrapped_text(
        ui,
        page,
        &encounter.translation,
        19.0,
        700,
        colors.text,
    )?];
    if !encounter.reading.is_empty() {
        title.push(text(
            ui,
            page,
            &encounter.reading,
            theme.small + 1.0,
            400,
            colors.secondary,
        )?);
    }
    let mut lines = vec![
        row(ui, page, 8.0, &title)?,
        wrapped_text(
            ui,
            page,
            &encounter.text,
            theme.font.size,
            400,
            colors.secondary,
        )?,
    ];
    if let Some(entry) = book.entry_of(encounter)
        && !entry.example.is_empty()
        && entry.example != encounter.text
    {
        lines.push(paragraph(ui, theme, &entry.example, None)?);
    }
    let content = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            gap: page.gap(3.0),
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
    let time = text(
        ui,
        page,
        &time_text(encounter),
        theme.small,
        400,
        colors.secondary,
    )?;
    let space = LengthPercentage::length(page.px(12.0));
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::FLEX_START,
            gap: page.gap(14.0),
            padding: Rect {
                top: space,
                bottom: space,
                left: LengthPercentage::length(0.0),
                right: LengthPercentage::length(0.0),
            },
            ..Style::default()
        },
        Visual::Group,
        &[dot, content, time],
    )
}

fn date_text(date: Date) -> String {
    tr_args(
        "timeline.date",
        &[
            ("year", &date.year()),
            ("month", &date.month()),
            ("day", &date.day()),
        ],
    )
}

/// 「下午 4:53」
fn time_text(encounter: &LoggedEncounter) -> String {
    let hour = encounter.time.hour();
    let half = if hour < 12 {
        tr("timeline.am")
    } else {
        tr("timeline.pm")
    };
    let twelve = match hour % 12 {
        0 => 12,
        other => other,
    };
    tr_args(
        "timeline.time",
        &[
            ("half", &half),
            ("hour", &twelve),
            ("minute", &format!("{:02}", encounter.time.minute())),
        ],
    )
}
