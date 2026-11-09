//! 统计页的「云服务」层：请求与 Token。

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{RenderError, Ui, group, section_title};
use qingjian_predict::ApiUsageHour;

use super::{card, cards, charts, detail, full_date, none_yet, note, requests_line, tokens};
use crate::i18n::{tr, tr_args};
use crate::message::Message;
use crate::stats::{CloudStats, group_digits, tokens_text};
use crate::view::Page;

pub fn cloud(
    ui: &mut Ui<Message>,
    page: &Page,
    stats: &CloudStats,
) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let mut out = Vec::new();

    let total = match &stats.totals {
        Some(totals) => card(
            ui,
            page,
            tr("stats.total_tokens"),
            &tokens_text(totals.total_tokens),
            &[
                tr_args(
                    "stats.request_count",
                    &[("n", &group_digits(totals.requests))],
                ),
                tr_args(
                    "stats.failure_count",
                    &[("n", &group_digits(totals.failures))],
                ),
            ],
        )?,
        None => card(
            ui,
            page,
            tr("stats.total_tokens"),
            "—",
            &[tr("stats.unreadable").to_owned()],
        )?,
    };
    let row = [
        hour_card(ui, page, tr("stats.today_tokens"), &stats.today_usage())?,
        hour_card(ui, page, tr("stats.week_tokens"), &stats.recent())?,
        hour_card(ui, page, tr("stats.month_tokens"), &stats.month())?,
        total,
    ];
    out.push(cards(ui, page, &row)?);

    out.push(section_title(ui, theme, tr("stats.section_year"))?);
    out.push(charts::calendar(
        ui,
        page,
        "cloud-days",
        stats.today,
        |date| stats.days.get(&date).map_or(0, |day| day.total_tokens),
        |date, tokens| {
            let requests = stats.days.get(&date).map_or(0, |day| day.requests);
            token_amount(tokens, requests)
        },
    )?);

    out.push(section_title(ui, theme, tr("stats.section_hours"))?);
    let hours = stats.hours;
    out.push(charts::hours(
        ui,
        page,
        "cloud-hours",
        stats.hour,
        hours.map(|hour| hour.total_tokens),
        |hour, tokens| token_amount(tokens, hours[hour].requests),
    )?);

    out.push(section_title(ui, theme, tr("stats.section_usage"))?);
    let best = stats.best_day().map_or(none_yet(), |(date, day)| {
        tr_args(
            "stats.day_tokens",
            &[
                ("date", &full_date(date)),
                ("n", &tokens_text(day.total_tokens)),
            ],
        )
    });
    let mut rows = vec![detail(ui, page, tr("stats.best_day"), None, &best)?];
    match &stats.totals {
        Some(totals) => {
            rows.push(note(ui, page, &requests_line(totals))?);
            rows.push(tokens(ui, page, totals)?);
        }
        None => rows.push(note(ui, page, tr("stats.usage_unreadable"))?),
    }
    out.push(group(ui, theme, &rows)?);
    Ok(out)
}

/// 数字卡：Token 数，下面是请求与失败次数。
fn hour_card(
    ui: &mut Ui<Message>,
    page: &Page,
    title: &str,
    usage: &ApiUsageHour,
) -> Result<NodeId, RenderError> {
    card(
        ui,
        page,
        title,
        &tokens_text(usage.total_tokens),
        &[
            tr_args(
                "stats.request_count",
                &[("n", &group_digits(usage.requests))],
            ),
            tr_args(
                "stats.failure_count",
                &[("n", &group_digits(usage.failures))],
            ),
        ],
    )
}

fn token_amount(tokens: u64, requests: u64) -> String {
    if requests == 0 {
        tr("stats.no_request").to_owned()
    } else {
        tr_args(
            "stats.hour_tokens",
            &[
                ("n", &tokens_text(tokens)),
                ("requests", &group_digits(requests)),
            ],
        )
    }
}
