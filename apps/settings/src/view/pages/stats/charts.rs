//! 统计页的两张图：近一年按天的格子图、过去 24 小时的柱状图，都装在卡片里，宽度跟着内容区走。

use bamboo_slip::taffy::{LengthPercentage, NodeId, Rect, Style};
use bamboo_slip::{BarChart, Datum, Heatmap, RenderError, Ui, Visual, group};
use jiff::civil::Date;

use crate::message::Message;
use crate::stats::week_start;
use crate::view::Page;

/// 格子图画几周（含这周）。
const WEEKS: i64 = 53;

/// 格子最大多大（点）。
const MAX_CELL: f32 = 14.0;

/// 柱状图的高（点）。
const BAR_HEIGHT: f32 = 96.0;

/// 卡片左右的内边距（点），见 `bamboo_slip::group`。
const CARD_PADDING: f32 = 16.0;

const WEEKDAYS: [&str; 7] = ["日", "一", "二", "三", "四", "五", "六"];

/// 近一年按天的格子图：一列一周（周日在上），颜色按那天的量相对这一年里最多那天的比例深浅。
pub fn calendar(
    ui: &mut Ui<Message>,
    page: &Page,
    local: &str,
    today: Date,
    value: impl Fn(Date) -> u64,
    tip: impl Fn(Date, u64) -> String,
) -> Result<NodeId, RenderError> {
    let start = week_start(today).saturating_sub(jiff::Span::new().weeks(WEEKS - 1));
    let dates: Vec<Date> = (0..WEEKS * 7)
        .map(|offset| start.saturating_add(jiff::Span::new().days(offset)))
        .collect();
    let max = dates
        .iter()
        .map(|date| value(*date))
        .max()
        .unwrap_or(0)
        .max(1);
    let mut columns = Vec::new();
    let mut labels = Vec::new();
    let mut last_label = None;
    for (week, days) in dates.chunks(7).enumerate() {
        // 这一列里有 1 号就标月份，离上一个月份标签太近（标不下）就跳过
        if let Some(first) = days.iter().find(|date| date.day() == 1 && **date <= today)
            && last_label.is_none_or(|last| week >= last + 3)
        {
            labels.push((week, format!("{}月", first.month())));
            last_label = Some(week);
        }
        columns.push(
            days.iter()
                .map(|date| {
                    (*date <= today).then(|| {
                        let amount = value(*date);
                        let level = (amount as f32 / max as f32).sqrt();
                        let weekday = WEEKDAYS[date.weekday().to_sunday_zero_offset() as usize];
                        let tip = format!(
                            "{}年{}月{}日 周{weekday} · {}",
                            date.year(),
                            date.month(),
                            date.day(),
                            tip(*date, amount)
                        );
                        Datum::new(if amount == 0 { 0.0 } else { level }, tip)
                    })
                })
                .collect(),
        );
    }
    let cell = Heatmap::fit(inner_width(page), WEEKS as usize).min(MAX_CELL);
    let rows = [1, 3, 5]
        .map(|row| (row, WEEKDAYS[row].to_owned()))
        .to_vec();
    let map = Heatmap::new(columns)
        .cell(cell)
        .column_labels(labels)
        .row_labels(rows)
        .build(ui, &page.theme, local)?;
    card(ui, page, map)
}

/// 过去 24 小时的柱状图：最后一根是现在这个钟点，画成满色；下面标起点、中间的 0、6、12、18 点和右头的「现在」。
pub fn hours(
    ui: &mut Ui<Message>,
    page: &Page,
    local: &str,
    now: u8,
    values: [u64; 24],
    tip: impl Fn(usize, u64) -> String,
) -> Result<NodeId, RenderError> {
    let max = values.iter().copied().max().unwrap_or(0).max(1);
    let clock = |index: usize| (usize::from(now) + 1 + index) % 24;
    let bars = values
        .iter()
        .enumerate()
        .map(|(index, amount)| {
            let hour = clock(index);
            let day = if 23 - index > usize::from(now) {
                "昨天 "
            } else {
                ""
            };
            Datum::new(
                *amount as f32 / max as f32,
                format!("{day}{hour}:00–{}:00 · {}", hour + 1, tip(index, *amount)),
            )
        })
        .collect();
    // 左头标起点，中间整 6 点，离两头太近的不标，右头是「现在」
    let labels = (0..24)
        .filter(|index| *index == 0 || (clock(*index) % 6 == 0 && (3..=20).contains(index)))
        .map(|index| (index, format!("{}:00", clock(index))))
        .collect();
    let chart = BarChart::new(bars, inner_width(page), BAR_HEIGHT)
        .labels(labels)
        .end_label("现在")
        .current(Some(23))
        .build(ui, &page.theme, local)?;
    card(ui, page, chart)
}

/// 卡片里图能用的宽度（点）。
fn inner_width(page: &Page) -> f32 {
    page.content_width / page.scale - CARD_PADDING * 2.0
}

/// 把图装进卡片，上下留白。
fn card(ui: &mut Ui<Message>, page: &Page, chart: NodeId) -> Result<NodeId, RenderError> {
    let space = LengthPercentage::length(page.px(14.0));
    let padded = ui.scene().node(
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
        &[chart],
    )?;
    group(ui, &page.theme, &[padded])
}
