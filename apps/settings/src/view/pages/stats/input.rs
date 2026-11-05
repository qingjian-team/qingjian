//! 统计页的「输入」层。

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{RenderError, SettingRow, Ui, group, section_title};
use qingjian_core::{Usage, VocabularySummary};
use qingjian_learning::figures::{
    best_day, clean_rate, daily_average, fastest_day, first_choice_rate, keys_per_hanzi,
};

use super::{
    card, cards, charts, detail, full_date, levels, none_yet, note, percent, vocabulary_line,
};
use crate::i18n::{tr, tr_args};
use crate::message::Message;
use crate::stats::{InputStats, group_digits, hanzi_text, scale_line};
use crate::view::Page;

pub fn input(
    ui: &mut Ui<Message>,
    page: &Page,
    stats: &InputStats,
    vocabulary: Option<&(&'static str, VocabularySummary)>,
) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let total = stats.total();
    let mut out = Vec::new();

    let row = [
        usage_card(ui, page, tr("stats.today"), &stats.today_usage())?,
        usage_card(ui, page, tr("stats.week"), &stats.recent())?,
        usage_card(ui, page, tr("stats.month"), &stats.month())?,
        usage_card(ui, page, tr("stats.total"), &total)?,
    ];
    out.push(cards(ui, page, &row)?);

    out.push(section_title(ui, theme, tr("stats.section_year"))?);
    out.push(charts::calendar(
        ui,
        page,
        "input-days",
        stats.today,
        |date| stats.on(date).hanzi,
        |_, hanzi| hanzi_amount(hanzi),
    )?);

    out.push(section_title(ui, theme, tr("stats.section_hours"))?);
    out.push(charts::hours(
        ui,
        page,
        "input-hours",
        stats.hour,
        stats.hours.map(|hour| hour.hanzi),
        |_, hanzi| hanzi_amount(hanzi),
    )?);

    out.push(section_title(ui, theme, tr("stats.section_input"))?);
    let rows = details(ui, page, stats, &total)?;
    out.push(group(ui, theme, &rows)?);

    match vocabulary {
        Some((language, vocabulary)) => {
            out.push(section_title(
                ui,
                theme,
                &tr_args("stats.vocabulary_title", &[("language", language)]),
            )?);
            let mut rows = vec![note(ui, page, &vocabulary_line(vocabulary))?];
            if !vocabulary.levels.is_empty() {
                rows.push(levels(ui, page, vocabulary)?);
            }
            out.push(group(ui, theme, &rows)?);
        }
        None => {
            out.push(section_title(ui, theme, tr("stats.section_vocabulary"))?);
            let off = note(ui, page, tr("stats.vocabulary_off"))?;
            out.push(group(ui, theme, &[off])?);
        }
    }
    Ok(out)
}

fn details(
    ui: &mut Ui<Message>,
    page: &Page,
    stats: &InputStats,
    total: &Usage,
) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let (days, since) = stats.active_days();
    let best = best_day(&stats.days).map_or(none_yet(), |(date, day)| {
        format!("{} · {}", full_date(date), hanzi_text(day.hanzi))
    });
    let average = daily_average(&stats.days).map_or(none_yet(), hanzi_text);
    let speed = |value: Option<f64>| {
        value.map_or(none_yet(), |v| {
            tr_args("stats.speed", &[("n", &format!("{v:.0}"))])
        })
    };
    let fastest = fastest_day(&stats.days).map_or(none_yet(), |(date, v)| {
        tr_args(
            "stats.fastest",
            &[("date", &full_date(date)), ("n", &format!("{v:.0}"))],
        )
    });
    let rate = |value: Option<f64>| value.map_or(none_yet(), percent);
    let code = keys_per_hanzi(total).map_or(none_yet(), |v| {
        tr_args("stats.keys_per_char", &[("n", &format!("{v:.1}"))])
    });
    let mut rows = vec![
        detail(ui, page, tr("stats.best_day"), None, &best)?,
        detail(ui, page, tr("stats.daily_average"), None, &average)?,
        detail(
            ui,
            page,
            tr("stats.today_speed"),
            Some(tr("stats.today_speed_info")),
            &speed(stats.today_speed()),
        )?,
        detail(
            ui,
            page,
            tr("stats.average_speed"),
            Some(tr("stats.average_speed_info")),
            &speed(stats.average_speed()),
        )?,
        detail(
            ui,
            page,
            tr("stats.fastest_day"),
            Some(tr("stats.fastest_info")),
            &fastest,
        )?,
        detail(
            ui,
            page,
            tr("stats.first_choice"),
            Some(tr("stats.first_choice_info")),
            &rate(first_choice_rate(total)),
        )?,
        detail(
            ui,
            page,
            tr("stats.clean"),
            Some(tr("stats.clean_info")),
            &rate(clean_rate(total)),
        )?,
        detail(
            ui,
            page,
            tr("stats.code_length"),
            Some(tr("stats.code_length_info")),
            &code,
        )?,
    ];
    let scale = scale_line(total.hanzi);
    let mut row = SettingRow::new(&scale).info(tr("stats.counting_info"));
    let since_line;
    if let Some(date) = since {
        since_line = tr_args("stats.since", &[("date", &date), ("days", &days)]);
        row = row.detail(&since_line);
    }
    rows.push(row.build(ui, theme)?);
    Ok(rows)
}

/// 数字卡：汉字数，下面是词数与上屏次数。
fn usage_card(
    ui: &mut Ui<Message>,
    page: &Page,
    title: &str,
    usage: &Usage,
) -> Result<NodeId, RenderError> {
    card(
        ui,
        page,
        title,
        &hanzi_text(usage.hanzi),
        &[
            tr_args("stats.chinese_words", &[("n", &group_digits(usage.words))]),
            tr_args(
                "stats.english_words",
                &[("n", &group_digits(usage.english_words))],
            ),
        ],
    )
}

fn hanzi_amount(hanzi: u64) -> String {
    if hanzi == 0 {
        tr("stats.no_input").to_owned()
    } else {
        hanzi_text(hanzi)
    }
}
