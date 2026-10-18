//! 统计页的「输入」层。

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{RenderError, SettingRow, Ui, group, section_title};
use qingjian_core::{Usage, VocabularySummary};

use super::{
    NONE_YET, card, cards, charts, detail, full_date, levels, note, percent, vocabulary_line,
};
use crate::message::Message;
use crate::stats::{
    InputStats, clean_rate, first_choice_rate, group_digits, hanzi_text, keys_per_hanzi, scale_line,
};
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
        usage_card(ui, page, "今天", &stats.today_usage())?,
        usage_card(ui, page, "本周", &stats.week())?,
        usage_card(ui, page, "本月", &stats.month())?,
        usage_card(ui, page, "累计", &total)?,
    ];
    out.push(cards(ui, page, &row)?);

    out.push(section_title(ui, theme, "近一年")?);
    out.push(charts::calendar(
        ui,
        page,
        "input-days",
        stats.today,
        |date| stats.on(date).hanzi,
        |_, hanzi| hanzi_amount(hanzi),
    )?);

    out.push(section_title(ui, theme, "过去 24 小时")?);
    out.push(charts::hours(
        ui,
        page,
        "input-hours",
        stats.hour,
        stats.hours.map(|hour| hour.hanzi),
        |_, hanzi| hanzi_amount(hanzi),
    )?);

    out.push(section_title(ui, theme, "输入详情")?);
    let rows = details(ui, page, stats, &total)?;
    out.push(group(ui, theme, &rows)?);

    match vocabulary {
        Some((language, vocabulary)) => {
            out.push(section_title(ui, theme, &format!("词汇（{language}）"))?);
            let mut rows = vec![note(ui, page, &vocabulary_line(vocabulary))?];
            if !vocabulary.levels.is_empty() {
                rows.push(levels(ui, page, vocabulary)?);
            }
            out.push(group(ui, theme, &rows)?);
        }
        None => {
            out.push(section_title(ui, theme, "词汇")?);
            let off = note(ui, page, "学习语言关着，不统计词汇")?;
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
    let best = stats.best_day().map_or(NONE_YET.to_owned(), |(date, day)| {
        format!("{} · {}", full_date(date), hanzi_text(day.hanzi))
    });
    let average = if days == 0 {
        NONE_YET.to_owned()
    } else {
        hanzi_text(total.hanzi / days as u64)
    };
    let speed = |value: Option<f64>| value.map_or(NONE_YET.to_owned(), |v| format!("{v:.0} 字/分"));
    let fastest = stats
        .fastest_day()
        .map_or(NONE_YET.to_owned(), |(date, v)| {
            format!("{} · {v:.0} 字/分", full_date(date))
        });
    let rate = |value: Option<f64>| value.map_or(NONE_YET.to_owned(), percent);
    let code = keys_per_hanzi(total).map_or(NONE_YET.to_owned(), |v| format!("{v:.1} 键/字"));
    const SPEED: &str = "上屏的汉字除以在打字的时间（词与词之间的间隔也算，停顿超过 5 秒的那段不算）；只算从这一版起选词打出的字";
    let mut rows = vec![
        detail(ui, page, "最多的一天", None, &best)?,
        detail(ui, page, "有输入的日子平均", None, &average)?,
        detail(
            ui,
            page,
            "今天速度",
            Some(SPEED),
            &speed(stats.today_speed()),
        )?,
        detail(
            ui,
            page,
            "平均速度",
            Some(SPEED),
            &speed(stats.average_speed()),
        )?,
        detail(ui, page, "最快的一天", Some(SPEED), &fastest)?,
        detail(
            ui,
            page,
            "首选率",
            Some("从候选里选词时，选的就是第一个的比例"),
            &rate(first_choice_rate(total)),
        )?,
        detail(
            ui,
            page,
            "一次打对",
            Some("选词上屏前没按过退格、没删过的比例"),
            &rate(clean_rate(total)),
        )?,
        detail(
            ui,
            page,
            "平均码长",
            Some("每个汉字平均敲几键，含拼音分隔符，不含退格"),
            &code,
        )?,
    ];
    let scale = scale_line(total.hanzi);
    let mut row = SettingRow::new(&scale).info(
        "数的是上屏的文字：选一个词算一个中文词，整句按词切开数；英文候选、原样上屏的英文词与英文译词算英文词。\
         速度、首选率、码长从这一版起记。只在这台电脑上数",
    );
    let since_line;
    if let Some(date) = since {
        since_line = format!("自 {date} 起，有输入的 {days} 天");
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
            format!("中文词 {}", group_digits(usage.words)),
            format!("英文词 {}", group_digits(usage.english_words)),
        ],
    )
}

fn hanzi_amount(hanzi: u64) -> String {
    if hanzi == 0 {
        "没有输入".to_owned()
    } else {
        hanzi_text(hanzi)
    }
}
