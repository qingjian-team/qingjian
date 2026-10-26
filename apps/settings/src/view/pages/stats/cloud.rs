//! 统计页的「云服务」层：请求与 Token。

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{RenderError, Ui, group, section_title};
use qingjian_predict::ApiUsageHour;

use super::{NONE_YET, card, cards, charts, detail, full_date, note, requests_line, tokens};
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
            "累计 Token",
            &tokens_text(totals.total_tokens),
            &[
                format!("请求 {} 次", group_digits(totals.requests)),
                format!("失败 {} 次", group_digits(totals.failures)),
            ],
        )?,
        None => card(ui, page, "累计 Token", "—", &["记录读不出来".to_owned()])?,
    };
    let row = [
        hour_card(ui, page, "今天 Token", &stats.today_usage())?,
        hour_card(ui, page, "近 7 天 Token", &stats.recent())?,
        hour_card(ui, page, "本月 Token", &stats.month())?,
        total,
    ];
    out.push(cards(ui, page, &row)?);

    out.push(section_title(ui, theme, "近一年")?);
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

    out.push(section_title(ui, theme, "过去 24 小时")?);
    let hours = stats.hours;
    out.push(charts::hours(
        ui,
        page,
        "cloud-hours",
        stats.hour,
        hours.map(|hour| hour.total_tokens),
        |hour, tokens| token_amount(tokens, hours[hour].requests),
    )?);

    out.push(section_title(ui, theme, "用量详情")?);
    let best = stats.best_day().map_or(NONE_YET.to_owned(), |(date, day)| {
        format!(
            "{} · {} Token",
            full_date(date),
            tokens_text(day.total_tokens)
        )
    });
    let mut rows = vec![detail(ui, page, "最多的一天", None, &best)?];
    match &stats.totals {
        Some(totals) => {
            rows.push(note(ui, page, &requests_line(totals))?);
            rows.push(tokens(ui, page, totals)?);
        }
        None => rows.push(note(
            ui,
            page,
            "用量记录读不出来，原有记录还在，稍后再进这一页看",
        )?),
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
            format!("请求 {} 次", group_digits(usage.requests)),
            format!("失败 {} 次", group_digits(usage.failures)),
        ],
    )
}

fn token_amount(tokens: u64, requests: u64) -> String {
    if requests == 0 {
        "没有请求".to_owned()
    } else {
        format!(
            "{} Token，请求 {} 次",
            tokens_text(tokens),
            group_digits(requests)
        )
    }
}
