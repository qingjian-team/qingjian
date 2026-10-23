//! 词库、码表的清单：每本一行（勾选开关、条数与许可证、移除），随包领域词库排成两列，末尾一行导入按钮。

use bamboo_slip::taffy::{
    AlignItems, Dimension, FlexDirection, FlexWrap, JustifyContent, LengthPercentage,
    LengthPercentageAuto, NodeId, Rect, Size, Style,
};
use bamboo_slip::{Button, Checkbox, RenderError, SettingRow, Ui, Visual, info_with_link};
use qingjian_dictionary::GENERAL_DICTIONARY;

use crate::library::{Entry, Kind};
use crate::message::Message;
use crate::view::Page;
use crate::view::parts::actions::with_notice;

/// 随包领域词库名字的统一前缀。
const DOMAIN_PREFIX: &str = "青简领域词库：";

/// 一本用户导入的词库或一套码表：`toggle` 给了就是勾选开关（是否开着、点了发什么），没给只写名字（码表在下拉框里选）；
/// 后面是条数与许可证，`removable` 的带「移除」。给了 `info` 的名字后面跟 ⓘ（链到文档），这一行也能搜到。
pub fn entry_row(
    ui: &mut Ui<Message>,
    page: &Page,
    kind: Kind,
    entry: &Entry,
    toggle: Option<(bool, Message)>,
    removable: bool,
    info: Option<&str>,
) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    let checkbox = match toggle {
        Some((on, toggle)) => Checkbox::new(on && !entry.broken, &entry.name)
            .disabled(entry.broken)
            .build(ui, theme, ("entry", kind, entry.stem.as_str()), toggle)?,
        None => ui.scene().node(
            Style::default(),
            Visual::Text {
                text: entry.name.clone(),
                style: theme.text(theme.font.size, 400, theme.colors.text),
            },
            &[],
        )?,
    };
    let checkbox = match info {
        Some(text) => {
            let key = ui.key(("row", entry.name.as_str()));
            ui.searchable(key, (&entry.name, Some(text)), &[], &[]);
            let link = ui.info_link(&entry.name);
            let mark = info_with_link(ui, theme, ("info", entry.name.as_str()), text, link)?;
            ui.scene().node(
                Style {
                    flex_direction: FlexDirection::Row,
                    align_items: Some(AlignItems::CENTER),
                    gap: page.gap(6.0),
                    ..Style::default()
                },
                Visual::Group,
                &[checkbox, mark],
            )?
        }
        None => checkbox,
    };
    let detail = if entry.broken {
        "文件坏了，打不开".to_owned()
    } else if entry.license.is_empty() {
        count(entry.entries)
    } else {
        format!("{} · {}", count(entry.entries), entry.license)
    };
    let detail = text(ui, page, &detail)?;
    let mut trailing = vec![detail];
    if removable {
        trailing.push(Button::new("移除").build(
            ui,
            theme,
            ("remove", kind, entry.stem.as_str()),
            Message::Remove(kind, entry.stem.clone()),
        )?);
    }
    let trailing = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::CENTER),
            gap: page.gap(12.0),
            ..Style::default()
        },
        Visual::Group,
        &trailing,
    )?;
    let space = LengthPercentage::length(page.px(10.0));
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::CENTER),
            justify_content: Some(JustifyContent::SPACE_BETWEEN),
            gap: page.gap(16.0),
            min_size: Size {
                width: LengthPercentageAuto::auto(),
                height: LengthPercentageAuto::length(page.px(theme.control_height + 20.0)),
            },
            padding: Rect {
                top: space,
                bottom: space,
                left: LengthPercentage::length(0.0),
                right: LengthPercentage::length(0.0),
            },
            ..Style::default()
        },
        Visual::Group,
        &[checkbox, trailing],
    )
}

/// 随包词库（通用词库与领域词库）：两列勾选，名字后面跟条数。
pub fn domain_grid(ui: &mut Ui<Message>, page: &Page) -> Result<NodeId, RenderError> {
    let config = &page.settings.config.dictionaries;
    let mut cells = Vec::new();
    for entry in &page.settings.library.domains {
        // 通用词库缺省开着，关掉是记进 disabled；领域词库缺省关着，打开是记进 domains
        let (on, message) = if entry.stem == GENERAL_DICTIONARY {
            (
                config.is_enabled(&entry.stem),
                toggle_disabled(Kind::Dictionary, &config.disabled, &entry.stem),
            )
        } else {
            let on = config.is_domain_enabled(&entry.stem);
            let domains: Vec<String> = if on {
                config
                    .domains
                    .iter()
                    .filter(|stem| **stem != entry.stem)
                    .cloned()
                    .collect()
            } else {
                config
                    .domains
                    .iter()
                    .cloned()
                    .chain(std::iter::once(entry.stem.clone()))
                    .collect()
            };
            (on, Message::SetList("dictionaries", "domains", domains))
        };
        // 随包词库的名字都带着这个前缀，页上已经分了组，去掉
        let name = entry
            .name
            .strip_prefix(DOMAIN_PREFIX)
            .unwrap_or(&entry.name);
        let checkbox = Checkbox::new(on && !entry.broken, name)
            .disabled(entry.broken)
            .build(ui, &page.theme, ("domain", entry.stem.as_str()), message)?;
        let detail = text(ui, page, &count(entry.entries))?;
        cells.push(ui.scene().node(
            Style {
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::CENTER),
                gap: page.gap(8.0),
                size: Size {
                    width: Dimension::percent(0.5),
                    height: Dimension::auto(),
                },
                ..Style::default()
            },
            Visual::Group,
            &[checkbox, detail],
        )?);
    }
    let gap = LengthPercentage::length(page.px(14.0));
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            gap: Size {
                width: LengthPercentage::length(0.0),
                height: gap,
            },
            padding: Rect {
                top: gap,
                bottom: gap,
                left: LengthPercentage::length(0.0),
                right: LengthPercentage::length(0.0),
            },
            ..Style::default()
        },
        Visual::Group,
        &cells,
    )
}

/// 末尾的导入行：按钮，导入中变灰，结果显示在下面。
pub fn import_row(
    ui: &mut Ui<Message>,
    page: &Page,
    kind: Kind,
    title: &str,
    info: &str,
) -> Result<NodeId, RenderError> {
    let running = page.settings.import.running() == Some(kind);
    let button = Button::new(if running {
        "导入中…"
    } else {
        "选择文件…"
    })
    .disabled(page.settings.import.running().is_some())
    .build(ui, &page.theme, ("import", kind), Message::Import(kind))?;
    with_notice(
        ui,
        page,
        SettingRow::new(title).info(info).control(button),
        kind.slot(),
    )
}

/// 用户导入的一本的开关：配置里记的是关掉的。
pub fn toggle_disabled(kind: Kind, disabled: &[String], stem: &str) -> Message {
    let list: Vec<String> = if disabled.iter().any(|name| name == stem) {
        disabled
            .iter()
            .filter(|name| *name != stem)
            .cloned()
            .collect()
    } else {
        disabled
            .iter()
            .cloned()
            .chain(std::iter::once(stem.to_owned()))
            .collect()
    };
    let (section, key) = kind.disabled_key();
    Message::SetList(section, key, list)
}

fn text(ui: &mut Ui<Message>, page: &Page, text: &str) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    ui.scene().node(
        Style::default(),
        Visual::Text {
            text: text.to_owned(),
            style: theme.text(theme.small, 400, theme.colors.secondary),
        },
        &[],
    )
}

/// 条数：上万的写成「3.2 万条」。
fn count(entries: usize) -> String {
    if entries >= 10_000 {
        let tens = (entries as f64 / 1000.0).round() / 10.0;
        format!("{tens} 万条")
    } else {
        format!("{entries} 条")
    }
}

#[cfg(test)]
mod tests {
    use super::count;

    #[test]
    fn counts_read_naturally() {
        assert_eq!(count(865), "865 条");
        assert_eq!(count(32_456), "3.2 万条");
        assert_eq!(count(87_000), "8.7 万条");
        assert_eq!(count(120_000), "12 万条");
    }
}
