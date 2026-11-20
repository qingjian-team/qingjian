//! Characterization 测试：锁住主题解析现状（扁平键集合、缺省值、部分覆盖、错误路径）。
//! 拆分重构前先跑绿；之后重构只能保持这些行为，原断言不弱化。

use serde_json::Value;

use bamboo_slip::Color;

use crate::theme::file::{ColorRef, ThemeFile};
use crate::theme::{Theme, ThemeError, jsonc};

/// 仓库内置的缺省主题：`status` / `keyboard` / `panel` 三节写全了，拿来当键集合与取值的基准。
const QINGJIAN: &str = include_str!("../../themes/qingjian/theme.json");

/// `panel` 一节用户可写的扁平键（拆分到子结构后用 `serde(flatten)` 保持这层键名不变）。
const PANEL_KEYS: &[&str] = &[
    "label_font",
    "hint_font",
    "item_font",
    "wide_item_font",
    "padding",
    "radius",
    "cell",
    "cell_gap",
    "cell_radius",
    "pill_padding_x",
    "pill_padding_y",
    "pill_gap",
    "tab_gap",
    "underline",
    "section_gap",
    "close_size",
    "background",
    "text",
    "label",
    "hint",
    "pill_fill",
    "pill_text",
    "underline_color",
    "divider",
    "highlight",
    "close_fill",
    "close_icon",
    "effects",
];

/// `keyboard` 一节用户可写的扁平键。
const KEYBOARD_KEYS: &[&str] = &[
    "key_font",
    "label_font",
    "root_font",
    "key_width",
    "key_height",
    "root_key_width",
    "root_key_height",
    "gap",
    "padding",
    "radius",
    "key_radius",
    "touch_radius",
    "touch_key_radius",
    "background",
    "key_fill",
    "key_text",
    "label",
    "initial",
    "pressed_fill",
    "pressed_text",
    "next_fill",
    "next_text",
    "dimmed_text",
    "effects",
];

/// `status` 一节用户可写的扁平键。
const STATUS_KEYS: &[&str] = &[
    "font",
    "padding",
    "radius",
    "background",
    "separator",
    "separator_width",
    "normal",
    "emphasized",
    "gear",
    "gear_size",
    "effects",
    "root",
];

fn strip_parse(source: &str) -> Value {
    serde_json::from_str(&jsonc::strip(source)).expect("主题示例应当是合法 JSON")
}

fn section_keys<'a>(value: &'a Value, section: &str) -> Vec<&'a str> {
    value
        .get(section)
        .and_then(Value::as_object)
        .map(|object| object.keys().map(String::as_str).collect())
        .unwrap_or_default()
}

fn key_paths(value: &Value, prefix: &str, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                key_paths(child, &path, out);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                key_paths(child, &format!("{prefix}[{index}]"), out);
            }
        }
        _ => out.push(prefix.to_owned()),
    }
}

#[test]
fn real_theme_section_keys_match_recorded_contract() {
    let value = strip_parse(QINGJIAN);
    let panel = section_keys(&value, "panel");
    let keyboard = section_keys(&value, "keyboard");
    let status = section_keys(&value, "status");
    // 真实主题写的键都是合同里认得的键；可选项（touch_*、root）可以不写。
    assert!(
        panel.iter().all(|key| PANEL_KEYS.contains(key)),
        "panel 出现合同外的键：{panel:?}"
    );
    assert!(
        keyboard.iter().all(|key| KEYBOARD_KEYS.contains(key)),
        "keyboard 出现合同外的键：{keyboard:?}"
    );
    assert!(
        status.iter().all(|key| STATUS_KEYS.contains(key)),
        "status 出现合同外的键：{status:?}"
    );
    // 合同键集合与结构体字段一一对应：面板 28、键位提示键盘 24、状态条 12。
    assert_eq!(PANEL_KEYS.len(), 28);
    assert_eq!(KEYBOARD_KEYS.len(), 24);
    assert_eq!(STATUS_KEYS.len(), 12);
}

#[test]
fn value_roundtrip_keeps_every_key_path() {
    let value = strip_parse(QINGJIAN);
    let serialized = serde_json::to_string(&value).expect("Value 应当能序列化");
    let again: Value = serde_json::from_str(&serialized).expect("回读应当成功");
    let mut before = Vec::new();
    let mut after = Vec::new();
    key_paths(&value, "", &mut before);
    key_paths(&again, "", &mut after);
    before.sort();
    after.sort();
    assert_eq!(before, after, "键路径在 Value 往返后丢了");
}

#[test]
fn real_theme_parses_into_file() {
    let value = strip_parse(QINGJIAN);
    let file: ThemeFile = serde_json::from_value(value).expect("内置青简绿应当解析成功");
    assert_eq!(file.meta.id, "qingjian");
    assert_eq!(file.variables.len(), 16);
    assert!(file.fonts.is_empty());
    assert!(file.material.is_none());
    assert_eq!(file.panel.effects.len(), 1);
    assert_eq!(file.keyboard.effects.len(), 1);
    assert_eq!(file.status.effects.len(), 1);
}

#[test]
fn unknown_top_level_and_section_keys_are_ignored() {
    let theme = Theme::from_json(
        r##"{
            "extends": "qingjian",
            "schema": 1,
            "meta": { "id": "unknown-keys", "name": "未知键" },
            "bogus_top_level": 1,
            "panel": { "bogus_panel_key": 2 },
            "variables": { "accent": "#ff0000" }
        }"##,
        false,
    )
    .unwrap();
    assert_eq!(theme.id(), "unknown-keys");
    assert_eq!(
        theme.color(&ColorRef::Variable("accent".to_owned())),
        Color::rgb(255, 0, 0)
    );
}

#[test]
fn missing_optional_parts_get_defaults() {
    let mut value = strip_parse(QINGJIAN);
    let object = value.as_object_mut().unwrap();
    object.remove("variables");
    object.remove("fonts");
    object.remove("components");
    object.remove("material");
    // 三节都去掉投影：缺省应为空数组
    for section in ["panel", "keyboard", "status"] {
        if let Some(map) = value.get_mut(section).and_then(Value::as_object_mut) {
            map.remove("effects");
        }
    }
    let file: ThemeFile = serde_json::from_value(value).unwrap();
    assert!(file.variables.is_empty());
    assert!(file.fonts.is_empty());
    assert!(file.components.is_empty());
    assert!(file.material.is_none());
    assert!(file.panel.effects.is_empty());
    assert!(file.keyboard.effects.is_empty());
    assert!(file.status.effects.is_empty());
}

#[test]
fn extends_partial_override_wins_and_rest_is_inherited() {
    let theme = Theme::from_json(
        r##"{
            "extends": "qingjian",
            "schema": 1,
            "meta": { "id": "override", "name": "覆盖" },
            "variables": { "accent": "#123456" }
        }"##,
        false,
    )
    .unwrap();
    assert_eq!(
        theme.color(&ColorRef::Variable("accent".to_owned())),
        Color::rgb(0x12, 0x34, 0x56)
    );
    // 没覆盖的变量继承青简绿的浅色值
    assert_eq!(
        theme.color(&ColorRef::Variable("fresh".to_owned())),
        Color::rgb(0xff, 0x8d, 0x28)
    );
}

#[test]
fn malformed_json_wrong_type_and_unknown_base_fail() {
    assert!(matches!(
        Theme::from_json("{ not json", false),
        Err(ThemeError::Json(_))
    ));
    // 错类型：panel.radius 写成字符串
    let wrong_type = Theme::from_json(
        r##"{
            "extends": "qingjian",
            "schema": 1,
            "meta": { "id": "wrong", "name": "错" },
            "panel": { "radius": "big" }
        }"##,
        false,
    );
    assert!(wrong_type.is_err(), "半径类型错应当报错");
    assert!(matches!(
        Theme::from_json(r#"{ "extends": "no-such-theme" }"#, false),
        Err(ThemeError::UnknownBase { .. })
    ));
}
