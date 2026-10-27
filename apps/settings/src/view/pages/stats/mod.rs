//! 「统计」页：顶上分「输入 / 云服务」两层，每层是四张数字卡（今天、近 7 天、本月、累计）、按天的格子图、过去 24 小时的柱状图和详情；
//! 输入层末尾是词汇（学习语言的译词见过、看熟、上屏过，有等级表时按级列）。云服务层只在记用量的平台显示。

mod charts;
mod cloud;
mod input;

use bamboo_slip::taffy::{
    AlignItems, Dimension, FlexDirection, JustifyContent, LengthPercentage, NodeId, Rect, Size,
    Style,
};
use bamboo_slip::{BoxPaint, Fill, RenderError, Segmented, SettingRow, Ui, Visual, paragraph};
use qingjian_core::VocabularySummary;
use qingjian_predict::ApiUsageStats;

use crate::message::Message;
use crate::stats::group_digits;
use crate::support::Feature;
use crate::view::Page;

/// 等级表的四列。
const LEVEL_COLUMNS: [&str; 4] = ["词表", "见过", "看熟", "上屏过"];

pub fn stats(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let Some(stats) = &page.settings.stats else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    let cloud = Feature::ApiUsage.here() && page.settings.stats_cloud;
    if Feature::ApiUsage.here() {
        let tabs = Segmented::new(&["输入", "云服务"], usize::from(cloud)).build(
            ui,
            &page.theme,
            "stats-tab",
            |index| Message::StatsCloud(index == 1),
        )?;
        out.push(ui.scene().node(
            Style {
                flex_direction: FlexDirection::Row,
                ..Style::default()
            },
            Visual::Group,
            &[tabs],
        )?);
    }
    if cloud {
        out.extend(cloud::cloud(ui, page, &stats.cloud)?);
    } else {
        out.extend(input::input(
            ui,
            page,
            &stats.input,
            stats.vocabulary.as_ref(),
        )?);
    }
    Ok(out)
}

/// 四张数字卡排成一行。
fn cards(ui: &mut Ui<Message>, page: &Page, cards: &[NodeId]) -> Result<NodeId, RenderError> {
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            gap: page.gap(10.0),
            ..Style::default()
        },
        Visual::Group,
        cards,
    )
}

/// 一张数字卡：标题、大字的数，下面几行小字。
fn card(
    ui: &mut Ui<Message>,
    page: &Page,
    title: &str,
    value: &str,
    lines: &[String],
) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    let colors = &theme.colors;
    let mut children = vec![
        text(ui, page, title, theme.small + 1.0, 600, colors.secondary)?,
        text(ui, page, value, 21.0, 600, colors.text)?,
    ];
    for line in lines {
        children.push(text(ui, page, line, theme.small, 400, colors.secondary)?);
    }
    let space = LengthPercentage::length(page.px(14.0));
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            flex_basis: Dimension::length(0.0),
            gap: page.gap(4.0),
            padding: Rect {
                left: space,
                right: space,
                top: space,
                bottom: space,
            },
            ..Style::default()
        },
        Visual::Box(BoxPaint {
            fill: Some(Fill::Solid(colors.surface)),
            border: Some((page.px(1.0), colors.line)),
            radius: page.px(theme.radius * 1.5),
        }),
        &children,
    )
}

/// 详情的一行：标题（可带 ⓘ）、右边的数。
fn detail(
    ui: &mut Ui<Message>,
    page: &Page,
    title: &str,
    info: Option<&str>,
    value: &str,
) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    let value = text(ui, page, value, theme.font.size, 500, theme.colors.text)?;
    let mut row = SettingRow::new(title).control(value);
    if let Some(info) = info {
        row = row.info(info);
    }
    row.build(ui, theme)
}

/// 0.75 → `75%`。
fn percent(rate: f64) -> String {
    format!("{:.0}%", rate * 100.0)
}

/// `2026年10月15日`：格子图能跨年，最多、最快的一天也可能在去年。
fn full_date(date: jiff::civil::Date) -> String {
    format!("{}年{}月{}日", date.year(), date.month(), date.day())
}

/// 还没有记录时详情里的数。
const NONE_YET: &str = "还没有记录";

/// 「请求 12 次，收到 11 次，失败 1 次」，还在等的、没报用量的另说。
fn requests_line(api: &ApiUsageStats) -> String {
    if api.requests == 0 {
        return "还没有发过请求".to_owned();
    }
    let mut line = format!(
        "请求 {} 次，收到 {} 次，失败 {} 次",
        group_digits(api.requests),
        group_digits(api.responses),
        group_digits(api.failures)
    );
    let unresolved = api.unresolved_requests();
    if unresolved > 0 {
        line += &format!("，还没有结果 {} 次", group_digits(unresolved));
    }
    if api.unreported_usage > 0 {
        line += &format!(
            "；{} 次服务没报用量，不在下面的数里",
            group_digits(api.unreported_usage)
        );
    }
    line
}

/// Token 表：表头一行，累计一行。缓存输入已经算在输入里。
fn tokens(ui: &mut Ui<Message>, page: &Page, api: &ApiUsageStats) -> Result<NodeId, RenderError> {
    let header = ["输入", "输出", "合计", "其中缓存"].map(str::to_owned);
    let values = [
        api.input_tokens,
        api.output_tokens,
        api.total_tokens,
        api.cached_input_tokens,
    ]
    .map(group_digits);
    let rows = [
        level_row(ui, page, "Token", &header, true)?,
        level_row(ui, page, "累计", &values, false)?,
    ];
    let space = LengthPercentage::length(page.px(12.0));
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            gap: page.gap(6.0),
            padding: Rect {
                top: space,
                bottom: space,
                left: LengthPercentage::length(0.0),
                right: LengthPercentage::length(0.0),
            },
            ..Style::default()
        },
        Visual::Group,
        &rows,
    )
}

/// 「见过 120 个词，看熟 80 个；上屏过 60 个，直接打出过 3 个；近 7 天新见 12 个」
fn vocabulary_line(summary: &VocabularySummary) -> String {
    if summary.seen == 0 {
        return "还没见过译词：打中文时候选右侧的译词就是词汇的来源".to_owned();
    }
    format!(
        "见过 {} 个词，看熟 {} 个；上屏过 {} 个，直接打出过 {} 个；近 7 天新见 {} 个",
        group_digits(summary.seen),
        group_digits(summary.familiar),
        group_digits(summary.committed),
        group_digits(summary.used),
        group_digits(summary.new_this_week)
    )
}

/// 按级的表：表头一行，一级一行。
fn levels(
    ui: &mut Ui<Message>,
    page: &Page,
    vocabulary: &VocabularySummary,
) -> Result<NodeId, RenderError> {
    let mut rows = Vec::new();
    let header: Vec<String> = LEVEL_COLUMNS
        .iter()
        .map(|title| (*title).to_owned())
        .collect();
    rows.push(level_row(ui, page, "等级", &header, true)?);
    for level in &vocabulary.levels {
        let values = [level.total, level.seen, level.familiar, level.committed].map(group_digits);
        rows.push(level_row(ui, page, &level.name, &values, false)?);
    }
    let space = LengthPercentage::length(page.px(12.0));
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            gap: page.gap(6.0),
            padding: Rect {
                top: space,
                bottom: space,
                left: LengthPercentage::length(0.0),
                right: LengthPercentage::length(0.0),
            },
            ..Style::default()
        },
        Visual::Group,
        &rows,
    )
}

fn level_row(
    ui: &mut Ui<Message>,
    page: &Page,
    name: &str,
    values: &[String],
    header: bool,
) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    let colors = &theme.colors;
    let (weight, color) = if header {
        (600, colors.secondary)
    } else {
        (400, colors.text)
    };
    let mut cells = vec![cell(ui, page, name, weight, color, false)?];
    for value in values {
        cells.push(cell(ui, page, value, weight, color, true)?);
    }
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            ..Style::default()
        },
        Visual::Group,
        &cells,
    )
}

/// 表格的一格：五格平分，数字靠右。
fn cell(
    ui: &mut Ui<Message>,
    page: &Page,
    value: &str,
    weight: u16,
    color: bamboo_slip::Color,
    number: bool,
) -> Result<NodeId, RenderError> {
    let label = text(ui, page, value, page.theme.small + 1.0, weight, color)?;
    ui.scene().node(
        Style {
            flex_grow: 1.0,
            flex_basis: Dimension::length(0.0),
            flex_direction: FlexDirection::Row,
            justify_content: if number {
                JustifyContent::FLEX_END
            } else {
                JustifyContent::FLEX_START
            },
            align_items: AlignItems::CENTER,
            size: Size {
                width: Dimension::auto(),
                height: Dimension::auto(),
            },
            ..Style::default()
        },
        Visual::Group,
        &[label],
    )
}

/// 卡片里的一段话。
fn note(ui: &mut Ui<Message>, page: &Page, text: &str) -> Result<NodeId, RenderError> {
    let note = paragraph(ui, &page.theme, text, Some(page.theme.colors.text))?;
    let space = LengthPercentage::length(page.px(12.0));
    ui.scene().node(
        Style {
            padding: Rect {
                top: space,
                bottom: space,
                left: LengthPercentage::length(0.0),
                right: LengthPercentage::length(0.0),
            },
            ..Style::default()
        },
        Visual::Group,
        &[note],
    )
}

fn text(
    ui: &mut Ui<Message>,
    page: &Page,
    text: &str,
    points: f32,
    weight: u16,
    color: bamboo_slip::Color,
) -> Result<NodeId, RenderError> {
    ui.scene().node(
        Style::default(),
        Visual::Text {
            text: text.to_owned(),
            style: page.theme.text(points, weight, color),
        },
        &[],
    )
}
