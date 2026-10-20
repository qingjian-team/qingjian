//! 各页共用的几种设置行：开关、分段选择、下拉框。

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{RenderError, Segmented, Select, SettingRow, Switch, TextField, Ui};

use super::menus::anchor;
use crate::message::{FieldId, Message, Popup};
use crate::view::Page;

/// 学习语言（配置写法、界面名），最后一项是不显示译文。
pub const LANGUAGES: [(&str, &str); 6] = [
    ("en", "英语"),
    ("ja", "日语"),
    ("es", "西班牙语"),
    ("vi", "越南语"),
    ("de", "德语"),
    (qingjian_platform::LEARNING_LANGUAGE_OFF, "不显示"),
];

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
    let switch = Switch::new(on).disabled(disabled).build(
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

/// 一组互斥取值的分段选择：`all` 按设置界面的顺序，`label` 取界面上的名字。
pub fn choice<T: Copy + PartialEq + 'static>(
    ui: &mut Ui<Message>,
    page: &Page,
    local: &str,
    all: &[T],
    current: T,
    label: fn(T) -> &'static str,
    message: fn(T) -> Message,
) -> Result<NodeId, RenderError> {
    let labels: Vec<&str> = all.iter().map(|value| label(*value)).collect();
    let selected = all.iter().position(|value| *value == current).unwrap_or(0);
    let values = all.to_vec();
    Segmented::new(&labels, selected)
        .build(ui, &page.theme, local, move |index| message(values[index]))
}

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
        .disabled(disabled)
        .build(ui, &page.theme, anchor(popup), Message::OpenMenu(popup))
}

/// 可保存、取消的单行配置输入框。
pub fn text_field(
    ui: &mut Ui<Message>,
    page: &Page,
    id: FieldId,
    title: &str,
    info: Option<&str>,
    placeholder: &str,
) -> Result<NodeId, RenderError> {
    let Some(edit) = page.settings.fields.get(&id) else {
        return SettingRow::new(title).build(ui, &page.theme);
    };
    let input = TextField::new(edit)
        .placeholder(placeholder)
        .secret(id == FieldId::ApiKey)
        .width(260.0)
        .build(
            ui,
            &page.theme,
            ("field", id),
            move |edit| Message::Edit(id, edit),
            Some(Message::Commit(id)),
            Some(Message::Cancel(id)),
        )?;
    let mut row = SettingRow::new(title).control(input);
    if let Some(info) = info {
        row = row.info(info);
    }
    row.build(ui, &page.theme)
}
