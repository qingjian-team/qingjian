//! 一张卡片：顶上色条（类型、时间、来源程序），中间内容，底下序号、字数与按钮。

use std::sync::Arc;

use bamboo_slip::taffy::{
    AlignItems, Dimension, FlexDirection, JustifyContent, NodeId, Size, Style,
};
use bamboo_slip::tiny_skia::Pixmap;
use bamboo_slip::{BoxPaint, Color, Easing, Fill, Interact, RenderError, Ui, Visual, tooltip};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use qingjian_clipboard::{ClipItem, ClipKind};

use super::body::body;
use super::icons::ActionGlyph;
use super::{Context, app_name, gap, pad, text_node};
use crate::app::Message;
use crate::i18n::{tr, tr_args};
use crate::palette::kind_color;

/// 卡片的宽（横排）与高（竖排）（点）。
const CARD_WIDTH: f32 = 260.0;

const CARD_HEIGHT: f32 = 200.0;

/// 色条与底栏大约占的高度（点），图片按剩下的地方缩放。
const CHROME_HEIGHT: f32 = 110.0;

const ICON_SIZE: f32 = 18.0;

/// 底栏图标按钮的边长（点）。
const ACTION_SIZE: f32 = 28.0;

/// 卡片圆角（点），同 Windows 11 的窗口与卡片。
const CARD_RADIUS: f32 = 8.0;

/// 卡片在滚动方向上的约束：横排时高度定死，竖排时宽度定死（像素）。
#[derive(Clone, Copy)]
pub(super) enum Extent {
    Tall(f32),

    Wide(f32),
}

/// 这一条的 key：选中项滚进视野时按它找。
pub(crate) fn row_key(item: &ClipItem) -> (&'static str, &str) {
    ("row", item.id.as_str())
}

/// 复制的时刻：一小时内按分钟，今天按小时，再早按日历算昨天、前天，今年写月日，更早带年份。
/// 不写「几秒前」：面板只在打开时刷新，秒数显示了也不会走。
fn age(updated_at: i64, now: i64, zone: &TimeZone) -> String {
    let minutes = (now - updated_at).max(0) / 60_000;
    if minutes < 1 {
        return tr("panel.just_now").to_owned();
    }
    if minutes < 60 {
        return tr_args("panel.minutes_ago", &[("count", &minutes)]);
    }
    let (Ok(at), Ok(today)) = (
        Timestamp::from_millisecond(updated_at),
        Timestamp::from_millisecond(now),
    ) else {
        return String::new();
    };
    let (at, today) = (at.to_zoned(zone.clone()), today.to_zoned(zone.clone()));
    let days = (today.date() - at.date()).get_days();
    match days {
        ..=0 => tr_args("panel.hours_ago", &[("count", &(minutes / 60))]),
        1 => tr("panel.yesterday").to_owned(),
        2 => tr("panel.day_before_yesterday").to_owned(),
        _ if at.year() == today.year() => at.strftime("%m-%d").to_string(),
        _ => at.strftime("%Y-%m-%d").to_string(),
    }
}

fn kind_label(kind: ClipKind) -> &'static str {
    match kind {
        ClipKind::Text => tr("kind.text"),
        ClipKind::Rich => tr("kind.rich"),
        ClipKind::Code => tr("kind.code"),
        ClipKind::Image => tr("kind.image"),
        ClipKind::Files => tr("kind.files"),
    }
}

fn column(
    ui: &mut Ui<Message>,
    align: AlignItems,
    children: &[NodeId],
) -> Result<NodeId, RenderError> {
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            align_items: align,
            ..Style::default()
        },
        Visual::Group,
        children,
    )
}

/// 图标垫一块白底：深色图标在彩色色条上看不清。
fn source_icon(
    ui: &mut Ui<Message>,
    context: &Context,
    icon: &Arc<Pixmap>,
) -> Result<NodeId, RenderError> {
    let theme = &context.theme;
    let side = theme.px(ICON_SIZE);
    let image = ui.scene().node(
        Style {
            size: Size {
                width: Dimension::length(side),
                height: Dimension::length(side),
            },
            ..Style::default()
        },
        Visual::Box(BoxPaint {
            fill: Some(Fill::Image {
                pixmap: icon.clone(),
                slice: None,
                pixels_per_px: 1.0,
            }),
            border: None,
            radius: 0.0,
        }),
        &[],
    )?;
    ui.scene().node(
        Style {
            padding: pad(theme, 2.0, 2.0),
            ..Style::default()
        },
        Visual::solid(Color::rgb(0xFF, 0xFF, 0xFF), theme.px(4.0)),
        &[image],
    )
}

fn header(ui: &mut Ui<Message>, context: &Context, item: &ClipItem) -> Result<NodeId, RenderError> {
    let theme = &context.theme;
    let on_accent = Color::rgb(0xFF, 0xFF, 0xFF);
    let kind = text_node(
        ui,
        kind_label(item.kind).to_owned(),
        theme.text(theme.font.size, 600, on_accent),
        Style::default(),
    )?;
    let time = text_node(
        ui,
        age(item.updated_at, context.now, &TimeZone::system()),
        theme.text(theme.small, 400, on_accent),
        Style::default(),
    )?;
    let left = column(ui, AlignItems::START, &[kind, time])?;
    let mut right = Vec::new();
    if let Some(Some(icon)) = context.app.icons.get(&item.source) {
        right.push(source_icon(ui, context, icon)?);
    }
    let name = app_name(item);
    if !name.is_empty() {
        right.push(text_node(
            ui,
            name,
            theme.text(theme.small, 400, on_accent),
            Style::default(),
        )?);
    }
    let right = column(ui, AlignItems::END, &right)?;
    ui.scene().node(
        Style {
            flex_shrink: 0.0,
            justify_content: JustifyContent::SPACE_BETWEEN,
            align_items: AlignItems::CENTER,
            padding: pad(theme, 6.0, 10.0),
            ..Style::default()
        },
        Visual::solid(kind_color(item.kind, context.app.dark), 0.0),
        &[left, right],
    )
}

/// 底栏的说明：文字是字数，图片是尺寸与大小，文件是项数，知道大小时带上大小。
fn measure(context: &Context, item: &ClipItem) -> String {
    match item.kind {
        ClipKind::Image => context
            .app
            .size_labels
            .get(&item.id)
            .cloned()
            .unwrap_or_default(),
        ClipKind::Files => {
            let count = item.text.lines().count();
            let items = tr_args("panel.file_count", &[("count", &count)]);
            match context.app.size_labels.get(&item.id) {
                Some(size) => format!("{items} · {size}"),
                None => items,
            }
        }
        _ => {
            let count = item.text.chars().count();
            tr_args("panel.characters", &[("count", &count)])
        }
    }
}

/// 底栏的图标按钮：平时没有底，悬停、按下才垫一层。
fn action(
    ui: &mut Ui<Message>,
    context: &Context,
    glyph: ActionGlyph,
    tip: &str,
    local: (&str, &str),
    on_click: Message,
) -> Result<NodeId, RenderError> {
    let theme = &context.theme;
    let colors = &theme.colors;
    let key = ui.key(local);
    let state = ui.state(key);
    let fill = if state.pressed {
        colors.pressed
    } else if state.hovered {
        colors.hover
    } else {
        colors.hover.transparent()
    };
    #[cfg(windows)]
    let icon = {
        let mut style = theme.text(theme.font.size, 400, colors.text);
        style.family = context.app.icon_font;
        text_node(ui, glyph.text().to_owned(), style, Style::default())?
    };
    #[cfg(not(windows))]
    let icon = {
        let side = theme.px(theme.font.size);
        ui.scene().node(
            Style {
                size: Size {
                    width: Dimension::length(side),
                    height: Dimension::length(side),
                },
                ..Style::default()
            },
            Visual::Icon {
                icon: glyph.icon(),
                size: side,
                color: colors.text,
            },
            &[],
        )?
    };
    let side = theme.px(ACTION_SIZE);
    let node = ui.scene().node(
        Style {
            size: Size {
                width: Dimension::length(side),
                height: Dimension::length(side),
            },
            flex_shrink: 0.0,
            justify_content: JustifyContent::CENTER,
            align_items: AlignItems::CENTER,
            ..Style::default()
        },
        Visual::solid(fill, theme.px(theme.radius)),
        &[icon],
    )?;
    ui.fade(node, key, theme.hover, Easing::Ease);
    ui.interact(node, key, Interact::new().on_click(on_click));
    if state.hovered {
        tooltip(ui, theme, key, tip, context.size)?;
    }
    Ok(node)
}

fn footer(
    ui: &mut Ui<Message>,
    context: &Context,
    item: &ClipItem,
    index: usize,
) -> Result<NodeId, RenderError> {
    let theme = &context.theme;
    let secondary = theme.colors.secondary;
    let number = text_node(
        ui,
        (index + 1).to_string(),
        theme.text(theme.small, 400, secondary),
        Style::default(),
    )?;
    let count = text_node(
        ui,
        measure(context, item),
        theme.text(theme.small, 400, secondary),
        Style::default(),
    )?;
    let paste = action(
        ui,
        context,
        ActionGlyph::Paste,
        tr("menu.paste"),
        ("paste", item.id.as_str()),
        Message::Paste(index),
    )?;
    let copy = action(
        ui,
        context,
        ActionGlyph::Copy,
        tr("menu.copy"),
        ("copy", item.id.as_str()),
        Message::Copy(index),
    )?;
    let buttons = ui.scene().node(
        Style {
            gap: gap(theme, 4.0),
            ..Style::default()
        },
        Visual::Group,
        &[paste, copy],
    )?;
    let info = ui.scene().node(
        Style {
            gap: gap(theme, 8.0),
            align_items: AlignItems::CENTER,
            ..Style::default()
        },
        Visual::Group,
        &[number, count],
    )?;
    ui.scene().node(
        Style {
            flex_shrink: 0.0,
            justify_content: JustifyContent::SPACE_BETWEEN,
            align_items: AlignItems::CENTER,
            padding: pad(theme, 4.0, 6.0),
            ..Style::default()
        },
        Visual::Group,
        &[info, buttons],
    )
}

pub(super) fn card(
    ui: &mut Ui<Message>,
    context: &Context,
    item: &ClipItem,
    index: usize,
    extent: Extent,
) -> Result<NodeId, RenderError> {
    let theme = &context.theme;
    let key = ui.key(row_key(item));
    let (size, room) = match extent {
        Extent::Tall(height) => (
            Size {
                width: Dimension::length(theme.px(CARD_WIDTH)),
                height: Dimension::length(height),
            },
            (theme.px(CARD_WIDTH), height),
        ),
        Extent::Wide(width) => (
            Size {
                width: Dimension::percent(1.0),
                height: Dimension::length(theme.px(CARD_HEIGHT)),
            },
            (width, theme.px(CARD_HEIGHT)),
        ),
    };
    let image_room = (
        room.0 - theme.px(20.0),
        (room.1 - theme.px(CHROME_HEIGHT)).max(theme.px(20.0)),
    );
    let children = [
        header(ui, context, item)?,
        body(ui, context, item, image_room)?,
        footer(ui, context, item, index)?,
    ];
    let colors = &theme.colors;
    let border = if index == context.app.selected {
        (theme.px(2.0), colors.accent)
    } else if ui.state(key).hovered {
        (theme.px(1.0), colors.secondary)
    } else {
        (theme.px(1.0), colors.line)
    };
    let radius = theme.px(CARD_RADIUS);
    let node = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            size,
            ..Style::default()
        },
        Visual::Box(BoxPaint {
            fill: Some(Fill::Solid(colors.surface)),
            border: Some(border),
            radius,
        }),
        &children,
    )?;
    ui.scene().set_clip(node, radius);
    ui.fade(node, key, theme.hover, Easing::Ease);
    ui.interact(
        node,
        key,
        Interact::new()
            .on_click(Message::Paste(index))
            .on_secondary(move |position| Message::OpenMenu(index, position)),
    );
    Ok(node)
}

#[cfg(test)]
mod tests {
    use jiff::Timestamp;
    use jiff::tz::TimeZone;

    use super::age;

    fn at(text: &str) -> i64 {
        text.parse::<Timestamp>().unwrap().as_millisecond()
    }

    #[test]
    fn ages_follow_the_calendar() {
        bamboo_slip_i18n::pin_locale("zh-Hans");
        let zone = TimeZone::UTC;
        let now = at("2026-11-02T10:00:00Z");
        for (then, expected) in [
            ("2026-11-02T09:59:30Z", "刚刚"),
            ("2026-11-02T09:15:00Z", "45 分钟前"),
            ("2026-11-02T01:00:00Z", "9 小时前"),
            ("2026-11-01T23:30:00Z", "昨天"),
            ("2026-11-01T01:00:00Z", "昨天"),
            ("2026-10-31T23:00:00Z", "前天"),
            ("2026-10-30T12:00:00Z", "10-30"),
            ("2025-12-31T12:00:00Z", "2025-12-31"),
        ] {
            assert_eq!(age(at(then), now, &zone), expected, "{then}");
        }
        // 刚过半夜：几十分钟前还是按分钟说，不说「昨天」
        let midnight = at("2026-11-02T00:10:00Z");
        assert_eq!(
            age(at("2026-11-01T23:50:00Z"), midnight, &zone),
            "20 分钟前"
        );
    }
}
