//! 各页共用的几种设置行：开关、分段选择、下拉框、百分比滑块。

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{Look, Menu, RenderError, Segmented, Select, SettingRow, Slider, Switch, Ui};

use super::menus::anchor;
use crate::i18n::tr;
use crate::message::{Message, Popup};
use crate::view::Page;

/// 一行开关：拨一下把 `[section] key` 写成相反的值。
pub fn toggle(
    ui: &mut Ui<Message>,
    page: &Page,
    title: &str,
    (section, key): (&'static str, &'static str),
    on: bool,
) -> Result<NodeId, RenderError> {
    toggle_with(ui, page, title, (section, key), on, false)
}

pub fn toggle_with(
    ui: &mut Ui<Message>,
    page: &Page,
    title: &str,
    key: (&'static str, &'static str),
    on: bool,
    disabled: bool,
) -> Result<NodeId, RenderError> {
    toggle_info(ui, page, title, None, key, on, disabled)
}

/// 带悬停说明的开关行。
pub fn toggle_info(
    ui: &mut Ui<Message>,
    page: &Page,
    title: &str,
    info: Option<&str>,
    (section, key): (&'static str, &'static str),
    on: bool,
    disabled: bool,
) -> Result<NodeId, RenderError> {
    let switch = switch(page, on).disabled(disabled).build(
        ui,
        &page.theme,
        (section, key),
        Message::set(section, key, !on),
    )?;
    let mut row = SettingRow::new(title).control(switch);
    if let Some(info) = info {
        row = row.info(info);
    }
    row.build(ui, &page.theme)
}

/// 开关：照 Windows 设置画时左边写「开 / 关」。
pub fn switch(page: &Page, on: bool) -> Switch<'static> {
    let switch = Switch::new(on);
    if page.theme.look == Look::Fluent {
        switch.labels(tr("common.on"), tr("common.off"))
    } else {
        switch
    }
}

/// 几选一的设置行：竹青与 macOS 是分段控件，Windows 照系统设置一律用下拉框（ComboBox），点开的菜单当场建在浮层里。
pub fn choice<T: Copy + PartialEq + 'static>(
    ui: &mut Ui<Message>,
    page: &Page,
    local: &'static str,
    all: &[T],
    current: T,
    label: fn(T) -> &'static str,
    message: fn(T) -> Message,
) -> Result<NodeId, RenderError> {
    let labels: Vec<&str> = all.iter().map(|value| label(*value)).collect();
    let selected = all.iter().position(|value| *value == current).unwrap_or(0);
    let values = all.to_vec();
    pick(ui, page, local, (&labels, selected), false, move |index| {
        message(values[index])
    })
}

/// 同 [`choice`]，选项是现成的字：`on_pick` 按选了第几项给消息。
pub fn pick(
    ui: &mut Ui<Message>,
    page: &Page,
    local: &'static str,
    (labels, selected): (&[&str], usize),
    disabled: bool,
    on_pick: impl Fn(usize) -> Message,
) -> Result<NodeId, RenderError> {
    if page.theme.look != Look::Fluent {
        return Segmented::new(labels, selected).disabled(disabled).build(
            ui,
            &page.theme,
            local,
            on_pick,
        );
    }
    dropdown(ui, page, local, (labels, selected), disabled, on_pick)
}

/// 不分外观的下拉框，点开的菜单当场建在浮层里；`local` 同时是菜单的名字。
pub fn dropdown(
    ui: &mut Ui<Message>,
    page: &Page,
    local: &'static str,
    (labels, selected): (&[&str], usize),
    disabled: bool,
    on_pick: impl Fn(usize) -> Message,
) -> Result<NodeId, RenderError> {
    let popup = Popup::Choice(local);
    let node = Select::new(labels[selected])
        .options(labels)
        .width(160.0)
        .disabled(disabled)
        .build(ui, &page.theme, anchor(popup), Message::OpenMenu(popup))?;
    if page.settings.menu == Some(popup)
        && let Some(rect) = ui.rect(ui.key(anchor(popup)))
    {
        let menu = Menu::new(labels, Some(selected), rect, page.window).build(
            ui,
            &page.theme,
            ("menu", local),
            on_pick,
            Message::CloseMenu,
        )?;
        ui.overlay(menu);
    }
    Ok(node)
}

/// 下拉框最宽多少点：值再长（用户方案起的名字）就截短补「…」，点开的菜单里仍是全名。
const SELECT_MAX_WIDTH: f32 = 260.0;

/// 下拉框，点开 `popup` 那个菜单；`options` 是菜单里的各项，只给搜索用。
pub fn select(
    ui: &mut Ui<Message>,
    page: &Page,
    popup: Popup,
    (value, options): (&str, &[&str]),
    width: f32,
    disabled: bool,
) -> Result<NodeId, RenderError> {
    Select::new(value)
        .options(options)
        .width(width)
        .max_width(SELECT_MAX_WIDTH)
        .disabled(disabled)
        .build(ui, &page.theme, anchor(popup), Message::OpenMenu(popup))
}

/// 一行百分比滑块，步长 5%：`ratio` 是当前倍数（1 为 100%），拖到的值按整数百分比写进 `[section] key`。
pub fn percent_slider(
    ui: &mut Ui<Message>,
    page: &Page,
    (title, (section, key)): (&str, (&'static str, &'static str)),
    ratio: f32,
    (min, max): (f32, f32),
    disabled: bool,
) -> Result<NodeId, RenderError> {
    let percent = (ratio * 100.0).round();
    let text = format!("{percent:.0}%");
    let slider = Slider::new(percent, min, max)
        .step(5.0)
        .label(&text)
        .disabled(disabled)
        .build(ui, &page.theme, key, move |value| {
            Message::set(section, key, value.round() as i64)
        })?;
    SettingRow::new(title)
        .control(slider)
        .build(ui, &page.theme)
}
